use minijinja::{
    value::{Value, ValueKind},
    ErrorKind,
};

pub struct SeqIterator {
    seq: Value,
    pos: usize,
    len: usize,
}

impl SeqIterator {
    pub fn new(seq: Value) -> Result<Self, minijinja::Error> {
        if !matches!(seq.kind(), ValueKind::Seq) {
            return Err(minijinja::Error::new(
                ErrorKind::ImpossibleOperation,
                "expected a list",
            ));
        }
        let len = seq.len().expect("Seq should have a length");
        Ok(Self { seq, pos: 0, len })
    }
}

impl Iterator for SeqIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.len {
            None
        } else {
            let item = self
                .seq
                .get_item(&Value::from(self.pos))
                .expect("item should be present at {self.pos}");
            self.pos += 1;
            Some(item)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}
