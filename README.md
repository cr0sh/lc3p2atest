# LC-3 Project #2A tester
주어진 오브젝트 파일이 LC-3 프로젝트 2A의 스펙(http://archi.snu.ac.kr/courses/under/19_spring_computer_concept/slides/proj2.pdf)을 만족하는지 [퍼즈 테스팅](https://ko.wikipedia.org/wiki/%ED%8D%BC%EC%A7%95)을 통해 확인합니다.

## 설치
- Windows 64bit: [Releases](https://github.com/cr0sh/lc3p2atest/releases)에서 실행 파일을 받을 수 있습니다.

- 기타 플랫폼: 소스로부터 직접 빌드해야 합니다.

### 직접 빌드
먼저, 컴파일러 툴체인의 설치는 [다음 링크](https://github.com/cr0sh/lc3dbg/blob/master/README_kr.md)의 `2. 직접 빌드-컴파일러 설치` 단락을 참고하세요.
**주의: 실습 서버(CCP)에서 직접 빌드하기 위해 컴파일러를 설치하면 많은 용량(1.6GB)를 차지하게 됩니다. 개인 환경에서만 사용하세요.**

소스코드를 받을 적절한 폴더 안에서, 다음 명령을 사용합니다. (`git` 설치 필요)

```shell
git clone https://github.com/cr0sh/lc3p2atest
cd lc3p2atest
cargo update
cargo run --release -- some/folder/proj2a.obj
```

처음으로 실행할 때는 패키지를 받고, 새로 컴파일하기 때문에 시간이 걸릴 수 있습니다.

## 기능
 - LC-3 시뮬레이터 환경([lc3-rs](https://github.com/cr0sh/lc3-rs)) 내장(`lc3sim` 설치 불필요)
 - 작은 크기의 데이터셋부터 큰 크기의 데이터셋까지 모두 테스트
 - 병렬화 지원(모든 CPU 자원을 최대로 활용)
 - VM의 메모리/레지스터를 랜덤화한 후 테스트(비-랜덤화 테스트 1회, 랜덤화 테스트 4회)
 - 테스트 실패 시 입력/출력/정답을 mismatch_*.txt로 출력

## 주의
 - 테스트 케이스의 수는 약 `15,0000 * 5 = 75,0000`개 정도입니다. 컴퓨터 성능에 따라 실행 시간이 많이 차이날 수 있으니 참고하세요.
 - CPU 자원(멀티코어의 경우 모두)을 최대한 활용하므로, 실행 시 부하가 큰 다른 프로그램(게임 등)을 끄고, 노트북의 경우 충전기를 연결하십시오.
