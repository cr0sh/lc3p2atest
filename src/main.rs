use conv::*;
use floating_duration::TimeFormat;
use heap::*;
use lc3::vm::{DSR, MCR, VM};
use rand::distributions::{Bernoulli, Standard, Uniform};
use rand::prelude::*;
use std::env::args;
use std::fs;
use std::io::{Result as IOResult, Write};
use std::iter;
use std::time::Instant;

#[cfg(feature = "parallel-test")]
use rayon::prelude::*;

mod heap;

// const HALT_MESSAGE: &str = "\n\n--- halting the LC-3 ---\n\n";
const OS_MINI: &[u8] = include_bytes!("static/lc3os_mini.obj");
const INSTRUCTION_LIMIT: usize = 5000_0000;
const ERR_DISPLAY_TRUNC_LIMIT: usize = 1000;
const RANDOMIZED_TESTS: usize = 4;

#[derive(Clone, Debug, PartialEq)]
enum Operation {
    Push(i16),
    Pop,
}

#[derive(Clone, Debug)]
struct SimpleTestCase {
    input: String,
    expect: String,
}

unsafe impl Send for SimpleTestCase {}

impl SimpleTestCase {
    fn test(self, mut vm: VM, limit: Option<usize>) -> Result<usize, SimpleTestError> {
        let mut out = Vec::<u8>::new();
        let instructions;
        if let Some(limit) = limit {
            instructions = vm.run_n_with_io(limit, &mut self.input.as_bytes(), &mut out);
        } else {
            instructions = vm.run_with_io(&mut self.input.as_bytes(), &mut out);
        }
        let output = String::from_utf8_lossy(&out);

        let alive = vm.mem[MCR] >> 15 > 0;
        let mismatch = output != self.expect;
        if alive || mismatch {
            return Err(SimpleTestError {
                input: self.input,
                output: output.into(),
                expect: self.expect,
                alive,
                mismatch,
            });
        }
        Ok(instructions)
    }
}

struct SimpleTestError {
    input: String,
    output: String,
    expect: String,
    alive: bool,
    mismatch: bool,
}

impl std::fmt::Display for SimpleTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}{}{}\n====입력====\n{}====출력====\n{}{}{}",
            if self.alive {
                "모든 입력 후 VM이 꺼지지 않음"
            } else {
                ""
            },
            if self.alive && self.mismatch {
                " + "
            } else {
                ""
            },
            if self.mismatch {
                "출력이 부적절함"
            } else {
                ""
            },
            if self.input.len() < ERR_DISPLAY_TRUNC_LIMIT {
                &self.input
            } else {
                "생략(너무 깁니다)\n"
            },
            if self.output.len() < ERR_DISPLAY_TRUNC_LIMIT {
                &self.output
            } else {
                "생략(너무 깁니다)\n"
            },
            if self.mismatch {
                "====정답====\n"
            } else {
                ""
            },
            if self.mismatch {
                if self.expect.len() < ERR_DISPLAY_TRUNC_LIMIT {
                    &self.expect
                } else {
                    "생략(너무 깁니다)\n"
                }
            } else {
                ""
            }
        )
    }
}

impl SimpleTestError {
    fn write_err(&self, input_path: &str, output_path: &str, expect_path: &str) -> IOResult<()> {
        fs::write(
            input_path,
            {
                format!(
                    "*** 다음 케이스를 실행한 결과 ***\n \
                     잘못된 정답 출력: {} \n \
                     VM이 꺼지지 않음: {} \n\n",
                    if self.mismatch { "YES" } else { "NO" },
                    if self.alive { "YES" } else { "NO" }
                ) + &self.input
            }
            .as_bytes(),
        )?;
        fs::write(output_path, self.output.as_bytes())?;
        fs::write(expect_path, self.expect.as_bytes())?;
        Ok(())
    }
}

struct FuzzyGenerator<N: Distribution<i16>, S: Distribution<usize>, O: Distribution<bool>> {
    num_range: N,
    size_range: S,
    element_count: usize,

    op_distribution: O, // If sampling is true, pop operation will be executed

    rng: ThreadRng,
}

impl<N: Distribution<i16>, S: Distribution<usize>, O: Distribution<bool>> Iterator
    for FuzzyGenerator<N, S, O>
{
    type Item = Vec<Operation>;

    fn next(&mut self) -> Option<Self::Item> {
        self.element_count = 0;
        let size: usize = self.rng.sample(&self.size_range);
        let mut v = Vec::with_capacity(size * 2);
        v.extend(
            iter::repeat_with(|| {
                if self.element_count > 0 && self.rng.sample(&self.op_distribution) {
                    if self.element_count > 0 {
                        self.element_count -= 1;
                    }
                    Operation::Pop
                } else {
                    if self.element_count < MAX_SIZE {
                        self.element_count += 1;
                    }
                    Operation::Push(self.rng.sample(&self.num_range))
                }
            })
            .take(size),
        );
        v.extend(iter::repeat(Operation::Pop).take(self.element_count));
        Some(v)
    }
}

#[allow(clippy::unused_io_amount)]
fn compile_ops(ops: Vec<Operation>) -> SimpleTestCase {
    let mut wr = Vec::new();
    let mut he = HeapEnv::new(&mut wr);

    ops.iter()
        .map(|op| match op {
            Operation::Push(num) => he.insert(*num),
            Operation::Pop => he.remove(),
        })
        .collect::<Result<(), _>>()
        .expect("Unexpected error while running heap environment");

    wr.write(b">q\n").unwrap();

    SimpleTestCase {
        input: ops
            .iter()
            .map(|op| match op {
                Operation::Push(num) => format!("i {}\n", num),
                Operation::Pop => String::from("r\n"),
            })
            .fold(String::new(), |a, b| a + &b)
            + "q\n",
        expect: String::from_utf8(wr.to_owned()).unwrap(),
    }
}

#[cfg(feature = "parallel-test")]
fn test_uniform<N: Distribution<i16>, S: Distribution<usize>, O: Distribution<bool>>(
    vm: &VM,
    num_range: N,
    size_range: S,
    op_distribution: O,
    n: usize,
    limit: Option<usize>,
) -> Result<f64, SimpleTestError> {
    let cases = (FuzzyGenerator {
        num_range,
        size_range,
        element_count: 0,
        op_distribution,
        rng: thread_rng(),
    })
    .take(n)
    .collect::<Vec<_>>();
    let cases_count = cases.len();
    Ok(cases
        .into_par_iter()
        .map(compile_ops)
        .map(|t| {
            t.test(vm.clone(), limit)
                .map(|x| f64::value_from(x).unwrap())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum::<f64>()
        / f64::value_from(cases_count).unwrap())
}

#[cfg(not(feature = "parallel-test"))]
fn test_uniform<N: Distribution<i16>, S: Distribution<usize>, O: Distribution<bool>>(
    vm: &VM,
    num_range: N,
    size_range: S,
    op_distribution: O,
    n: usize,
    limit: Option<usize>,
) -> Result<f64, SimpleTestError> {
    let cases = (FuzzyGenerator {
        num_range,
        size_range,
        element_count: 0,
        op_distribution,
        rng: thread_rng(),
    })
    .take(n)
    .collect::<Vec<_>>();
    let cases_count = cases.len();
    Ok(cases
        .into_iter()
        .map(compile_ops)
        .map(|t| {
            t.test(vm.clone(), limit)
                .map(|x| f64::value_from(x).unwrap())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum::<f64>()
        / f64::value_from(cases_count).unwrap())
}

fn main() {
    println!("LC-3 project #2A tester v{}", env!("CARGO_PKG_VERSION"));
    println!("이 프로그램은 GPL-2 라이선스 하에서 배포되며,");
    println!(
        "내장된 LC-3 운영체제 OS_MINI에 대한 원 저작권은 Steven S. Lumetta에게 있습니다.\n"
    );
    let f = fs::read(
        args()
            .nth(1)
            .expect("이 프로그램은 .obj 파일을 필요로 합니다"),
    )
    .expect("Object 파일을 열 수 없습니다");

    let mut vm = VM::default();
    vm.load_u8(OS_MINI);
    vm.load_u8(&f);

    let randomize_vm = |vm: &mut VM| {
        thread_rng().fill(&mut vm.mem[..]);
        thread_rng().fill(&mut vm.register[..]);
        vm.mem[DSR] = 0b1000_0000_0000_0000;
        vm.mem[MCR] = 0b1000_0000_0000_0000;
        vm.load_u8(OS_MINI);
        vm.load_u8(&f);
    };

    macro_rules! err_print {
        ($err: expr) => {
            println!("\t테스트 실패: {}", $err);
            $err.write_err("mismatch_input.txt", "mismatch_output.txt", "mismatch_expect.txt").expect("실패한 테스트 케이스를 기록하지 못했습니다");
            println!("\t해당 정보는 mismatch_*.txt 파일에 기록되었습니다.");
            println!("\thttps://www.diffchecker.com 과 같은 사이트에서 정답(expect)과 실제 출력(output)의 차이점을 분석해보세요.");
        }
    }

    macro_rules! test_case {
        ($name: expr, $v:expr, [$nr0:expr, $nr1:expr], [$sr0:expr, $sr1:expr], $opdist:expr, $n:expr) => {
            test_case!(@explicit $name, $v, [$nr0, $nr1], [$sr0, $sr1], $opdist, $n, Some(INSTRUCTION_LIMIT))
        };

        ($name: expr, $v:expr, [$nr0:expr, $nr1:expr], [$sr0:expr, $sr1:expr], $opdist:expr, $n:expr, @nolimit) => {
            test_case!(@explicit $name, $v, [$nr0, $nr1], [$sr0, $sr1], $opdist, $n, None)
        };

        (@explicit $name: expr, $v:expr, [$nr0:expr, $nr1:expr], [$sr0:expr, $sr1:expr], $opdist:expr, $n:expr, $limit:expr) => {
            println!(
                "{}개짜리 테스트 세트 [{}]에 대해 테스트를 시작합니다",
                $n, $name
            );
            println!("\t입력 데이터의 값 구간: [{}, {}]", $nr0, $nr1);
            println!("\t입력 데이터의 개수 구간: [{}, {}]", $sr0, $sr1);

            let now = Instant::now();

            match test_uniform(
                $v,
                Uniform::new_inclusive($nr0, $nr1),
                Uniform::new_inclusive($sr0, $sr1),
                $opdist,
                $n,
                $limit,
            ) {
                Ok(x) => {
                    let dur = now.elapsed();
                    println!(
                        "\t테스트 성공: 총 {}, 각 테스트당 평균 {}의 시간, {:.2} instruction이 들었습니다.",
                        TimeFormat(dur),
                        TimeFormat(dur / $n),
                        x
                    );
                },
                Err(err) =>{
                err_print!(err);
                return;
                },
            };

        };
    }

    fn test_group(vm: &mut VM) {
        test_case!("simple", &vm, [0, 3], [3, 5], Standard, 100);
        test_case!("duplicates", &vm, [0, 5], [200, 200], Standard, 5000);
        test_case!("all_same", &vm, [1, 1], [30, 30], Standard, 30);
        test_case!("zero_one", &vm, [0, 1], [50, 50], Standard, 200);
        test_case!("zero_one_two", &vm, [0, 2], [80, 80], Standard, 500);
        test_case!(
            "1/3_pop_2/3_insert",
            &vm,
            [0, 20],
            [200, 250],
            Bernoulli::from_ratio(1, 3),
            500
        );
        test_case!(
            "2/3_pop_1/3_insert",
            &vm,
            [0, 20],
            [200, 250],
            Bernoulli::from_ratio(2, 3),
            500
        );
        test_case!("small", &vm, [0, 50], [10, 20], Standard, 10_0000);
        test_case!("medium", &vm, [0, 200], [50, 60], Standard, 1_0000);
        test_case!("large", &vm, [0, 7000], [300, 350], Standard, 2500, @nolimit);
        test_case!("xlarge", &vm, [0, i16::max_value()], [7500, 8000], Standard, 200, @nolimit);
    };

    let start = Instant::now();
    println!("** 기본 테스트 그룹에 대해 테스트를 시작합니다.");
    test_group(&mut vm);
    println!("** 기본 테스트 그룹의 모든 테스트를 통과했습니다.");

    println!("** 랜덤화 테스트 그룹에 대해 테스트를 시작합니다.");
    for i in 1..=RANDOMIZED_TESTS {
        println!("** 테스트 전 LC-3 VM의 레지스터와 메모리를 랜덤화합니다.");
        randomize_vm(&mut vm);

        println!(
            "** 랜덤화 테스트 그룹 {}/{}에 대해 테스트를 시작합니다.",
            i, RANDOMIZED_TESTS
        );
        test_group(&mut vm);
        println!(
            "** 랜덤화 테스트 그룹 {}의 모든 테스트를 통과했습니다.",
            i
        );
    }

    println!("** 모든 테스트 그룹을 통과했습니다.");
    println!("** 총 소요된 시간: {}", TimeFormat(start.elapsed()));
}
