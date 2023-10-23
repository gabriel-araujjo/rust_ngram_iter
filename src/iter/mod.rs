#[cfg(test)]
mod bigram_test;
#[cfg(test)]
mod trigram_test;

use ringbuffer::{RingBuffer, ConstGenericRingBuffer};

use crate::{state::State, Iterable};
use std::mem::{MaybeUninit, transmute_copy};

/// An iterator over arbitrary-N-grams of arbitrary `Copy` types `T`.
///
/// `N` must be greater than or equal to 2, and this **is not** verified at
/// compile-time.
///
/// ```
/// use ngram_iter::Iterable; // adds the `bumper_item()` function to char.
/// let letters: String = ('a'..='z').collect();
/// let mut bigrams: ngram_iter::Iter<_, _, 2> = letters.chars().into();
/// assert_eq!(bigrams.next(), Some([char::bumper_item(), 'a']));
/// assert_eq!(bigrams.next(), Some(['a', 'b']));
/// let mut trigrams: ngram_iter::Iter<_, _, 3> = letters.chars().into();
/// assert_eq!(trigrams.next(), Some([char::bumper_item(), 'a', 'b']));
/// let mut ten_grams: ngram_iter::Iter<_, _, 10> = letters.chars().into();
/// assert_eq!(ten_grams.next(), Some([char::bumper_item(), 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i']));
/// assert_eq!(ten_grams.next(), Some(['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j']));
///
/// // N < 2 is panics at runtime!
/// let mut one_gram: ngram_iter::Iter<_, _, 1> = letters.chars().into();
/// std::panic::catch_unwind(move || one_gram.next()).expect_err("ngram with N<2 panics at runtime");
/// ```
pub struct Iter<T, I, const N: usize>
where
    T: Copy + Iterable,
    I: Iterator<Item = T>,
{
    it: I,
    state: State<T, N>,
}

impl<T, I, const N: usize> Iterator for Iter<T, I, N>
where
    T: Iterable + Copy,
    I: Iterator<Item = T>,
{
    type Item = [T; N];

    /// Returns a list of the next N items.
    ///
    /// panics if N < 2!
    fn next(&mut self) -> Option<Self::Item> {
        if N < 2 {
            panic!("ngram must have N of 2 or more")
        }
        let mut out: [MaybeUninit<T>; N] = unsafe {
            MaybeUninit::uninit().assume_init()
        };
        match self.state {
            State::Start => {
                let mut rb = ConstGenericRingBuffer::new();
                if let Some(item) = self.it.next() {
                    // dependent iterator has a least one item
                    rb.push(item);
                    out[0] = MaybeUninit::new(T::bumper_item());
                    out[1] = MaybeUninit::new(item);
                } else {
                    // Iterating over empty iterator, skip to the end.
                    self.state = State::End;
                    return None;
                }
                // Fill in the remaining values if N is greater than 2.
                for i in 2..N {
                    if let Some(item) = self.it.next() {
                        rb.push(item);
                        out[i] = MaybeUninit::new(item);
                    } else {
                        // Fill in N-i values with the bumper/buffer item.
                        for j in i..N {
                            out[j] = MaybeUninit::new(T::bumper_item());
                        }
                    }
                }
                self.state = State::Middle(rb);
            }
            State::Middle(ref mut rb) => {
                // first value was stored in the state.
                let mut i = 0;
                out[i] = MaybeUninit::new(rb.dequeue().unwrap());
                i += 1;

                for c in rb.iter() {
                    out[i] = MaybeUninit::new(*c);
                    i += 1;
                }

                if let Some(item) = self.it.next() {
                    rb.push(item);
                    out[i] = MaybeUninit::new(item);
                    i += 1;
                }
                for j in i..N {
                    out[j] = MaybeUninit::new(T::bumper_item());
                }
                if rb.is_empty() {
                    self.state = State::End;
                }
            }
            State::End => return None,
        }
        Some(unsafe { transmute_copy(&out) })
    }
}

impl<T, I, const N: usize> From<I> for Iter<T, I, N>
where
    T: Copy + Iterable,
    I: Iterator<Item = T>,
{
    fn from(it: I) -> Self {
        Self {
            it,
            state: State::Start,
        }
    }
}
