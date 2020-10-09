#![allow(dead_code)]

#[derive(Copy, Clone, Debug)]
pub struct MemoryRange {
    pub min : usize,
    pub max : usize
}

impl MemoryRange {
    pub fn empty() -> Self {
        Self {
            min : usize::MAX,
            max : 0
        }
    }
    
    pub fn new(min : usize, max : usize) -> Self {
        if max <= min {
            Self::empty()
        } else {
            Self {
                min,
                max
            }
        }
    }

    pub fn is_empty(&mut self) -> bool {
        self.max < self.min
    }   

    pub fn set_empty(&mut self) {
        self.max = 0;
        self.min = usize::MAX;
    }

    pub fn include_int(&mut self, x : usize) {
        self.min = self.min.min(x);       
        self.max = self.max.max(x + 1);
    }

    pub fn include_range(&mut self, other : MemoryRange){
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
}

use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

impl From<usize> for MemoryRange {
    fn from(a : usize) -> Self {
        Self::new(a, a+1)
    }
}

impl From<Range<usize>> for MemoryRange {
    fn from(a : Range<usize>) -> Self {
        Self::new(a.start, a.end + 1)
    }
}

impl From<RangeFrom<usize>> for MemoryRange {
    fn from(a : RangeFrom<usize>) -> Self {
        Self::new(a.start, usize::MAX)
    }
}

impl From<RangeFull> for MemoryRange {
    fn from(_a : RangeFull) -> Self {
        Self::new(0, usize::MAX)
    }
}

impl From<RangeInclusive<usize>> for MemoryRange {
    fn from(a : RangeInclusive<usize>) -> Self {
        Self::new(*a.start(), *a.end() + 1)
    }
}

impl From<RangeTo<usize>> for MemoryRange {
    fn from(a : RangeTo<usize>) -> Self {
        Self::new(0, a.end)
    }
}

impl From<RangeToInclusive<usize>> for MemoryRange {
    fn from(a : RangeToInclusive<usize>) -> Self {
        Self::new(0, a.end + 1)
    }
}