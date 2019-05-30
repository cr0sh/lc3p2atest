use std::io::{Result as IOResult, Write};

pub const MAX_SIZE: usize = 20;

pub struct HeapEnv<W> {
    pub arr: [i16; MAX_SIZE],
    pub size: usize,
    pub wr: W,
}

#[allow(clippy::unused_io_amount)]
impl<W> HeapEnv<W>
where
    W: Write,
{
    pub fn new(wr: W) -> HeapEnv<W> {
        HeapEnv {
            arr: [0; MAX_SIZE],
            size: 0,
            wr,
        }
    }

    pub fn insert(&mut self, item: i16) -> IOResult<()> {
        self.wr.write(format!(">i {}\n", item).as_bytes())?;
        let mut curr;
        let mut next;

        if self.size == MAX_SIZE {
            return self.wr.write(b"Insert failed\n").map(|_| ());
        }

        self.arr[self.size] = item;
        self.size += 1;

        curr = self.size - 1;
        next = ((curr as isize - 1) / 2) as usize;
        self.list()?;

        while curr != 0 {
            if self.arr[curr] < self.arr[next] {
                self.arr[curr] = self.arr[next];
                self.arr[next] = item;
                curr = next;
                next = ((curr as isize - 1) / 2) as usize;
                self.list()?;
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn remove(&mut self) -> IOResult<()> {
        self.wr.write(b">r\n")?;
        let mut curr;
        let mut next;

        if self.size == 0 {
            return self.wr.write(b"Remove failed\n").map(|_| ());
        }

        self.arr[0] = self.arr[self.size - 1];
        self.size -= 1;

        curr = 0;
        next = 2 * curr + 1;
        self.list()?;
        while next < self.size {
            if next + 1 < self.size && self.arr[next + 1] < self.arr[next] {
                next += 1;
            }
            if self.arr[next] < self.arr[curr] {
                self.arr.swap(curr, next);
                curr = next;
                next = 2 * curr + 1;
                self.list()?;
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn list(&mut self) -> IOResult<()>
    where
        W: Write,
    {
        self.wr.write(b"Heap: ")?;
        if self.size == 0 {
            self.wr.write(b"Empty")?;
        }

        for idx in 0..self.size {
            self.wr.write(format!("{} ", self.arr[idx]).as_bytes())?;
        }

        self.wr.write(b"\n")?;
        Ok(())
    }
}
