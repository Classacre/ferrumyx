//! Batched inference utilities.

use std::collections::VecDeque;

pub struct BatchProcessor<T> {
    buffer: VecDeque<T>,
    batch_size: usize,
}

impl<T> BatchProcessor<T> {
    pub fn new(batch_size: usize) -> Self {
        Self { buffer: VecDeque::with_capacity(batch_size), batch_size }
    }

    pub fn push(&mut self, item: T) -> Option<Vec<T>> {
        self.buffer.push_back(item);
        if self.buffer.len() >= self.batch_size { self.flush() } else { None }
    }

    pub fn flush(&mut self) -> Option<Vec<T>> {
        if self.buffer.is_empty() { None } else { Some(self.buffer.drain(..).collect()) }
    }
}

pub struct BatchIterator<T, I> where I: Iterator<Item = T> {
    source: I,
    buffer: Vec<T>,
    batch_size: usize,
}

impl<T, I> BatchIterator<T, I> where I: Iterator<Item = T> {
    pub fn new(source: I, batch_size: usize) -> Self {
        Self { source, buffer: Vec::with_capacity(batch_size), batch_size }
    }
}

impl<T, I> Iterator for BatchIterator<T, I> where I: Iterator<Item = T> {
    type Item = Vec<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.clear();
        while self.buffer.len() < self.batch_size {
            match self.source.next() {
                Some(item) => self.buffer.push(item),
                None => break,
            }
        }
        if self.buffer.is_empty() { None } else { Some(std::mem::take(&mut self.buffer)) }
    }
}
