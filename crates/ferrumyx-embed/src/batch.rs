//! Batched inference utilities.

use std::collections::VecDeque;

/// Batch processor for efficient inference.
///
/// Accumulates inputs and processes them in batches,
/// reducing overhead from model calls.
pub struct BatchProcessor<T> {
    buffer: VecDeque<T>,
    batch_size: usize,
}

impl<T> BatchProcessor<T> {
    /// Create a new batch processor.
    pub fn new(batch_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(batch_size),
            batch_size,
        }
    }

    /// Add an item to the buffer.
    /// Returns Some(batch) if the buffer is full and ready to process.
    pub fn push(&mut self, item: T) -> Option<Vec<T>> {
        self.buffer.push_back(item);
        
        if self.buffer.len() >= self.batch_size {
            self.flush()
        } else {
            None
        }
    }

    /// Flush remaining items in buffer.
    pub fn flush(&mut self) -> Option<Vec<T>> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(self.buffer.drain(..).collect())
        }
    }

    /// Get current buffer size.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// Process items in batches, yielding results as they become available.
pub struct BatchIterator<T, I>
where
    I: Iterator<Item = T>,
{
    source: I,
    buffer: Vec<T>,
    batch_size: usize,
}

impl<T, I> BatchIterator<T, I>
where
    I: Iterator<Item = T>,
{
    pub fn new(source: I, batch_size: usize) -> Self {
        Self {
            source,
            buffer: Vec::with_capacity(batch_size),
            batch_size,
        }
    }
}

impl<T, I> Iterator for BatchIterator<T, I>
where
    I: Iterator<Item = T>,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.clear();
        
        while self.buffer.len() < self.batch_size {
            match self.source.next() {
                Some(item) => self.buffer.push(item),
                None => break,
            }
        }
        
        if self.buffer.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.buffer))
        }
    }
}

/// Chunk a slice into batches of specified size.
pub fn chunk_slice<T>(slice: &[T], batch_size: usize) -> Vec<&[T]> {
    slice.chunks(batch_size).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_processor() {
        let mut processor = BatchProcessor::new(3);
        
        // First two items don't trigger flush
        assert!(processor.push(1).is_none());
        assert!(processor.push(2).is_none());
        
        // Third item triggers flush
        let batch = processor.push(3);
        assert_eq!(batch, Some(vec![1, 2, 3]));
        
        // Remaining items
        assert!(processor.push(4).is_none());
        let remaining = processor.flush();
        assert_eq!(remaining, Some(vec![4]));
    }

    #[test]
    fn test_batch_iterator() {
        let items = vec![1, 2, 3, 4, 5, 6, 7];
        let batches: Vec<Vec<i32>> = BatchIterator::new(items.into_iter(), 3).collect();
        
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0], vec![1, 2, 3]);
        assert_eq!(batches[1], vec![4, 5, 6]);
        assert_eq!(batches[2], vec![7]);
    }
}
