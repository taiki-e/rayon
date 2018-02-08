//! Traits for writing parallel programs using an iterator-style interface
//!
//! You will rarely need to interact with this module directly unless you have
//! need to name one of the iterator types.
//!
//! Parallel iterators make it easy to write iterator-like chains that
//! execute in parallel: typically all you have to do is convert the
//! first `.iter()` (or `iter_mut()`, `into_iter()`, etc) method into
//! `par_iter()` (or `par_iter_mut()`, `into_par_iter()`, etc). For
//! example, to compute the sum of the squares of a sequence of
//! integers, one might write:
//!
//! ```rust
//! use rayon::prelude::*;
//! fn sum_of_squares(input: &[i32]) -> i32 {
//!     input.par_iter()
//!          .map(|i| i * i)
//!          .sum()
//! }
//! ```
//!
//! Or, to increment all the integers in a slice, you could write:
//!
//! ```rust
//! use rayon::prelude::*;
//! fn increment_all(input: &mut [i32]) {
//!     input.par_iter_mut()
//!          .for_each(|p| *p += 1);
//! }
//! ```
//!
//! To use parallel iterators, first import the traits by adding
//! something like `use rayon::prelude::*` to your module. You can
//! then call `par_iter`, `par_iter_mut`, or `into_par_iter` to get a
//! parallel iterator. Like a [regular iterator][], parallel
//! iterators work by first constructing a computation and then
//! executing it.
//!
//! In addition to `par_iter()` and friends, some types offer other
//! ways to create (or consume) parallel iterators:
//!
//! - Slices (`&[T]`, `&mut [T]`) offer methods like `par_split` and
//!   `par_windows`, as well as various parallel sorting
//!   operations. See [the `ParallelSlice` trait] for the full list.
//! - Strings (`&str`) offer methods like `par_split` and `par_lines`.
//!   See [the `ParallelString` trait] for the full list.
//! - Various collections offer [`par_extend`], which grows a
//!   collection given a parallel iterator. (If you don't have a
//!   collection to extend, you can use [`collect()`] to create a new
//!   one from scratch.)
//!
//! [the `ParallelSlice` trait]: ../slice/trait.ParallelSlice.html
//! [the `ParallelString` trait]: ../str/trait.ParallelString.html
//! [`par_extend`]: trait.ParallelExtend.html
//! [`collect()`]: trait.ParallelIterator.html#method.collect
//!
//! To see the full range of methods available on parallel iterators,
//! check out the [`ParallelIterator`] and [`IndexedParallelIterator`]
//! traits.
//!
//! If you'd like to offer parallel iterators for your own collector,
//! or write your own combinator, then check out the [plumbing]
//! module.
//!
//! [regular iterator]: http://doc.rust-lang.org/std/iter/trait.Iterator.html
//! [`ParallelIterator`]: trait.ParallelIterator.html
//! [`IndexedParallelIterator`]: trait.IndexedParallelIterator.html
//! [plumbing]: plumbing

pub use either::Either;
use std::cmp::{self, Ordering};
use std::iter::{Sum, Product};
use std::ops::Fn;
use self::plumbing::*;

// There is a method to the madness here:
//
// - Most of these modules are private but expose certain types to the end-user
//   (e.g., `enumerate::Enumerate`) -- specifically, the types that appear in the
//   public API surface of the `ParallelIterator` traits.
// - In **this** module, those public types are always used unprefixed, which forces
//   us to add a `pub use` and helps identify if we missed anything.
// - In contrast, items that appear **only** in the body of a method,
//   e.g. `find::find()`, are always used **prefixed**, so that they
//   can be readily distinguished.

mod find;
mod find_first_last;
mod chain;
pub use self::chain::Chain;
mod chunks;
pub use self::chunks::Chunks;
mod collect;
mod enumerate;
pub use self::enumerate::Enumerate;
mod filter;
pub use self::filter::Filter;
mod filter_map;
pub use self::filter_map::FilterMap;
mod flat_map;
pub use self::flat_map::FlatMap;
mod flatten;
pub use self::flatten::Flatten;
mod from_par_iter;
pub mod plumbing;
mod for_each;
mod fold;
pub use self::fold::{Fold, FoldWith};
mod reduce;
mod skip;
pub use self::skip::Skip;
mod splitter;
pub use self::splitter::{split, Split};
mod take;
pub use self::take::Take;
mod map;
pub use self::map::Map;
mod map_with;
pub use self::map_with::MapWith;
mod zip;
pub use self::zip::Zip;
mod zip_eq;
pub use self::zip_eq::ZipEq;
mod interleave;
pub use self::interleave::Interleave;
mod interleave_shortest;
pub use self::interleave_shortest::InterleaveShortest;
mod intersperse;
pub use self::intersperse::Intersperse;
mod update;
pub use self::update::Update;

mod noop;
mod rev;
pub use self::rev::Rev;
mod len;
pub use self::len::{MinLen, MaxLen};
mod sum;
mod product;
mod cloned;
pub use self::cloned::Cloned;
mod inspect;
pub use self::inspect::Inspect;
mod while_some;
pub use self::while_some::WhileSome;
mod extend;
mod unzip;
mod repeat;
pub use self::repeat::{Repeat, repeat};
pub use self::repeat::{RepeatN, repeatn};

mod empty;
pub use self::empty::{Empty, empty};
mod once;
pub use self::once::{Once, once};

#[cfg(test)]
mod test;

/// `IntoParallelIterator` implements the conversion to a [`ParallelIterator`].
///
/// By implementing `IntoParallelIterator` for a type, you define how it will
/// transformed into an iterator. This is a parallel version of the standard
/// library's [`std::iter::IntoIterator`] trait.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`std::iter::IntoIterator`]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html
pub trait IntoParallelIterator {
    /// The parallel iterator type that will be created.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    type Item: Send;

    /// Converts `self` into a parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// println!("counting in parallel:");
    /// (0..100).into_par_iter()
    ///     .for_each(|i| println!("{}", i));
    /// ```
    ///
    /// This conversion is often implicit for arguments to methods like [`zip`].
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let v: Vec<_> = (0..5).into_par_iter().zip(5..10).collect();
    /// assert_eq!(v, [(0, 5), (1, 6), (2, 7), (3, 8), (4, 9)]);
    /// ```
    ///
    /// [`zip`]: trait.IndexedParallelIterator.html#method.zip
    fn into_par_iter(self) -> Self::Iter;
}

/// `IntoParallelRefIterator` implements the conversion to a
/// [`ParallelIterator`], providing shared references to the data.
///
/// This is a parallel version of the `iter()` method
/// defined by various collections.
///
/// This trait is automatically implemented
/// `for I where &I: IntoParallelIterator`. In most cases, users
/// will want to implement [`IntoParallelIterator`] rather than implement
/// this trait directly.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`IntoParallelIterator`]: trait.IntoParallelIterator.html
pub trait IntoParallelRefIterator<'data> {
    /// The type of the parallel iterator that will be returned.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    /// This will typically be an `&'data T` reference type.
    type Item: Send + 'data;

    /// Converts `self` into a parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let v: Vec<_> = (0..100).collect();
    /// assert_eq!(v.par_iter().sum::<i32>(), 100 * 99 / 2);
    ///
    /// // `v.par_iter()` is shorthand for `(&v).into_par_iter()`,
    /// // producing the exact same references.
    /// assert!(v.par_iter().zip(&v)
    ///          .all(|(a, b)| std::ptr::eq(a, b)));
    /// ```
    fn par_iter(&'data self) -> Self::Iter;
}

impl<'data, I: 'data + ?Sized> IntoParallelRefIterator<'data> for I
    where &'data I: IntoParallelIterator
{
    type Iter = <&'data I as IntoParallelIterator>::Iter;
    type Item = <&'data I as IntoParallelIterator>::Item;

    fn par_iter(&'data self) -> Self::Iter {
        self.into_par_iter()
    }
}


/// `IntoParallelRefMutIterator` implements the conversion to a
/// [`ParallelIterator`], providing mutable references to the data.
///
/// This is a parallel version of the `iter_mut()` method
/// defined by various collections.
///
/// This trait is automatically implemented
/// `for I where &mut I: IntoParallelIterator`. In most cases, users
/// will want to implement [`IntoParallelIterator`] rather than implement
/// this trait directly.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`IntoParallelIterator`]: trait.IntoParallelIterator.html
pub trait IntoParallelRefMutIterator<'data> {
    /// The type of iterator that will be created.
    type Iter: ParallelIterator<Item = Self::Item>;

    /// The type of item that will be produced; this is typically an
    /// `&'data mut T` reference.
    type Item: Send + 'data;

    /// Creates the parallel iterator from `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut v = vec![0usize; 5];
    /// v.par_iter_mut().enumerate().for_each(|(i, x)| *x = i);
    /// assert_eq!(v, [0, 1, 2, 3, 4]);
    /// ```
    fn par_iter_mut(&'data mut self) -> Self::Iter;
}

impl<'data, I: 'data + ?Sized> IntoParallelRefMutIterator<'data> for I
    where &'data mut I: IntoParallelIterator
{
    type Iter = <&'data mut I as IntoParallelIterator>::Iter;
    type Item = <&'data mut I as IntoParallelIterator>::Item;

    fn par_iter_mut(&'data mut self) -> Self::Iter {
        self.into_par_iter()
    }
}

/// Parallel version of the standard iterator trait.
///
/// The combinators on this trait are available on **all** parallel
/// iterators.  Additional methods can be found on the
/// [`IndexedParallelIterator`] trait: those methods are only
/// available for parallel iterators where the number of items is
/// known in advance (so, e.g., after invoking `filter`, those methods
/// become unavailable).
///
/// For examples of using parallel iterators, see [the docs on the
/// `iter` module][iter].
///
/// [iter]: index.html
/// [`IndexedParallelIterator`]: trait.IndexedParallelIterator.html
pub trait ParallelIterator: Sized + Send {
    /// The type of item that this parallel iterator produces.
    /// For example, if you use the [`for_each`] method, this is the type of
    /// item that your closure will be invoked with.
    ///
    /// [`for_each`]: #method.for_each
    type Item: Send;

    /// Executes `OP` on each item produced by the iterator, in parallel.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// (0..100).into_par_iter().for_each(|x| println!("{:?}", x));
    /// ```
    fn for_each<OP>(self, op: OP)
        where OP: Fn(Self::Item) + Sync + Send
    {
        for_each::for_each(self, &op)
    }

    /// Executes `OP` on the given `init` value with each item produced by
    /// the iterator, in parallel.
    ///
    /// The `init` value will be cloned only as needed to be paired with
    /// the group of items in each rayon job.  It does not require the type
    /// to be `Sync`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::mpsc::channel;
    /// use rayon::prelude::*;
    ///
    /// let (sender, receiver) = channel();
    ///
    /// (0..5).into_par_iter().for_each_with(sender, |s, x| s.send(x).unwrap());
    ///
    /// let mut res: Vec<_> = receiver.iter().collect();
    ///
    /// res.sort();
    ///
    /// assert_eq!(&res[..], &[0, 1, 2, 3, 4])
    /// ```
    fn for_each_with<OP, T>(self, init: T, op: OP)
        where OP: Fn(&mut T, Self::Item) + Sync + Send,
              T: Send + Clone
    {
        self.map_with(init, op).for_each(|()| ())
    }

    /// Counts the number of items in this parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let count = (0..100).into_par_iter().count();
    ///
    /// assert_eq!(count, 100);
    /// ```
    fn count(self) -> usize {
        self.map(|_| 1).sum()
    }

    /// Applies `map_op` to each item of this iterator, producing a new
    /// iterator with the results.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut par_iter = (0..5).into_par_iter().map(|x| x * 2);
    ///
    /// let doubles: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&doubles[..], &[0, 2, 4, 6, 8]);
    /// ```
    fn map<F, R>(self, map_op: F) -> Map<Self, F>
        where F: Fn(Self::Item) -> R + Sync + Send,
              R: Send
    {
        map::new(self, map_op)
    }

    /// Applies `map_op` to the given `init` value with each item of this
    /// iterator, producing a new iterator with the results.
    ///
    /// The `init` value will be cloned only as needed to be paired with
    /// the group of items in each rayon job.  It does not require the type
    /// to be `Sync`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::mpsc::channel;
    /// use rayon::prelude::*;
    ///
    /// let (sender, receiver) = channel();
    ///
    /// let a: Vec<_> = (0..5)
    ///                 .into_par_iter()            // iterating over i32
    ///                 .map_with(sender, |s, x| {
    ///                     s.send(x).unwrap();     // sending i32 values through the channel
    ///                     x                       // returning i32
    ///                 })
    ///                 .collect();                 // collecting the returned values into a vector
    ///
    /// let mut b: Vec<_> = receiver.iter()         // iterating over the values in the channel
    ///                             .collect();     // and collecting them
    /// b.sort();
    ///
    /// assert_eq!(a, b);
    /// ```
    fn map_with<F, T, R>(self, init: T, map_op: F) -> MapWith<Self, T, F>
        where F: Fn(&mut T, Self::Item) -> R + Sync + Send,
              T: Send + Clone,
              R: Send
    {
        map_with::new(self, init, map_op)
    }

    /// Creates an iterator which clones all of its elements.  This may be
    /// useful when you have an iterator over `&T`, but you need `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3];
    ///
    /// let v_cloned: Vec<_> = a.par_iter().cloned().collect();
    ///
    /// // cloned is the same as .map(|&x| x), for integers
    /// let v_map: Vec<_> = a.par_iter().map(|&x| x).collect();
    ///
    /// assert_eq!(v_cloned, vec![1, 2, 3]);
    /// assert_eq!(v_map, vec![1, 2, 3]);
    /// ```
    fn cloned<'a, T>(self) -> Cloned<Self>
        where T: 'a + Clone + Send,
              Self: ParallelIterator<Item = &'a T>
    {
        cloned::new(self)
    }

    /// Applies `inspect_op` to a reference to each item of this iterator,
    /// producing a new iterator passing through the original items.  This is
    /// often useful for debugging to see what's happening in iterator stages.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 4, 2, 3];
    ///
    /// // this iterator sequence is complex.
    /// let sum = a.par_iter()
    ///             .cloned()
    ///             .filter(|&x| x % 2 == 0)
    ///             .reduce(|| 0, |sum, i| sum + i);
    ///
    /// println!("{}", sum);
    ///
    /// // let's add some inspect() calls to investigate what's happening
    /// let sum = a.par_iter()
    ///             .cloned()
    ///             .inspect(|x| println!("about to filter: {}", x))
    ///             .filter(|&x| x % 2 == 0)
    ///             .inspect(|x| println!("made it through filter: {}", x))
    ///             .reduce(|| 0, |sum, i| sum + i);
    ///
    /// println!("{}", sum);
    /// ```
    fn inspect<OP>(self, inspect_op: OP) -> Inspect<Self, OP>
        where OP: Fn(&Self::Item) + Sync + Send
    {
        inspect::new(self, inspect_op)
    }

    /// Mutates each item of this iterator before yielding it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let par_iter = (0..5).into_par_iter().update(|x| {*x *= 2;});
    ///
    /// let doubles: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&doubles[..], &[0, 2, 4, 6, 8]);
    /// ```
    fn update<F>(self, update_op: F) -> Update<Self, F>
        where F: Fn(&mut Self::Item) + Sync + Send
    {
        update::new(self, update_op)
    }

    /// Applies `filter_op` to each item of this iterator, producing a new
    /// iterator with only the items that gave `true` results.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut par_iter = (0..10).into_par_iter().filter(|x| x % 2 == 0);
    ///
    /// let even_numbers: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&even_numbers[..], &[0, 2, 4, 6, 8]);
    /// ```
    fn filter<P>(self, filter_op: P) -> Filter<Self, P>
        where P: Fn(&Self::Item) -> bool + Sync + Send
    {
        filter::new(self, filter_op)
    }

    /// Applies `filter_op` to each item of this iterator to get an `Option`,
    /// producing a new iterator with only the items from `Some` results.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut par_iter = (0..10).into_par_iter()
    ///                         .filter_map(|x| {
    ///                             if x % 2 == 0 { Some(x * 3) }
    ///                             else { None }
    ///                         });
    ///
    /// let even_numbers: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&even_numbers[..], &[0, 6, 12, 18, 24]);
    /// ```
    fn filter_map<P, R>(self, filter_op: P) -> FilterMap<Self, P>
        where P: Fn(Self::Item) -> Option<R> + Sync + Send,
              R: Send
    {
        filter_map::new(self, filter_op)
    }

    /// Applies `map_op` to each item of this iterator to get nested iterators,
    /// producing a new iterator that flattens these back into one.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [[1, 2], [3, 4], [5, 6], [7, 8]];
    ///
    /// let par_iter = a.par_iter().cloned().flat_map(|a| a.to_vec());
    ///
    /// let vec: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&vec[..], &[1, 2, 3, 4, 5, 6, 7, 8]);
    /// ```
    fn flat_map<F, PI>(self, map_op: F) -> FlatMap<Self, F>
        where F: Fn(Self::Item) -> PI + Sync + Send,
              PI: IntoParallelIterator
    {
        flat_map::new(self, map_op)
    }

    /// An adaptor that flattens iterable `Item`s into one large iterator
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let x: Vec<Vec<_>> = vec![vec![1, 2], vec![3, 4]];
    /// let y: Vec<_> = x.into_par_iter().flatten().collect();
    ///
    /// assert_eq!(y, vec![1, 2, 3, 4]);
    /// ```
    fn flatten(self) -> Flatten<Self>
        where Self::Item: IntoParallelIterator
    {
        flatten::new(self)
    }

    /// Reduces the items in the iterator into one item using `op`.
    /// The argument `identity` should be a closure that can produce
    /// "identity" value which may be inserted into the sequence as
    /// needed to create opportunities for parallel execution. So, for
    /// example, if you are doing a summation, then `identity()` ought
    /// to produce something that represents the zero for your type
    /// (but consider just calling `sum()` in that case).
    ///
    /// # Examples
    ///
    /// ```
    /// // Iterate over a sequence of pairs `(x0, y0), ..., (xN, yN)`
    /// // and use reduce to compute one pair `(x0 + ... + xN, y0 + ... + yN)`
    /// // where the first/second elements are summed separately.
    /// use rayon::prelude::*;
    /// let sums = [(0, 1), (5, 6), (16, 2), (8, 9)]
    ///            .par_iter()        // iterating over &(i32, i32)
    ///            .cloned()          // iterating over (i32, i32)
    ///            .reduce(|| (0, 0), // the "identity" is 0 in both columns
    ///                    |a, b| (a.0 + b.0, a.1 + b.1));
    /// assert_eq!(sums, (0 + 5 + 16 + 8, 1 + 6 + 2 + 9));
    /// ```
    ///
    /// **Note:** unlike a sequential `fold` operation, the order in
    /// which `op` will be applied to reduce the result is not fully
    /// specified. So `op` should be [associative] or else the results
    /// will be non-deterministic. And of course `identity()` should
    /// produce a true identity.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    fn reduce<OP, ID>(self, identity: ID, op: OP) -> Self::Item
        where OP: Fn(Self::Item, Self::Item) -> Self::Item + Sync + Send,
              ID: Fn() -> Self::Item + Sync + Send
    {
        reduce::reduce(self, identity, op)
    }

    /// Reduces the items in the iterator into one item using `op`.
    /// If the iterator is empty, `None` is returned; otherwise,
    /// `Some` is returned.
    ///
    /// This version of `reduce` is simple but somewhat less
    /// efficient. If possible, it is better to call `reduce()`, which
    /// requires an identity element.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// let sums = [(0, 1), (5, 6), (16, 2), (8, 9)]
    ///            .par_iter()        // iterating over &(i32, i32)
    ///            .cloned()          // iterating over (i32, i32)
    ///            .reduce_with(|a, b| (a.0 + b.0, a.1 + b.1))
    ///            .unwrap();
    /// assert_eq!(sums, (0 + 5 + 16 + 8, 1 + 6 + 2 + 9));
    /// ```
    ///
    /// **Note:** unlike a sequential `fold` operation, the order in
    /// which `op` will be applied to reduce the result is not fully
    /// specified. So `op` should be [associative] or else the results
    /// will be non-deterministic.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    fn reduce_with<OP>(self, op: OP) -> Option<Self::Item>
        where OP: Fn(Self::Item, Self::Item) -> Self::Item + Sync + Send
    {
        self.fold(|| None, |opt_a, b| match opt_a {
                Some(a) => Some(op(a, b)),
                None => Some(b),
            })
            .reduce(|| None, |opt_a, opt_b| match (opt_a, opt_b) {
                (Some(a), Some(b)) => Some(op(a, b)),
                (Some(v), None) | (None, Some(v)) => Some(v),
                (None, None) => None,
            })
    }

    /// Parallel fold is similar to sequential fold except that the
    /// sequence of items may be subdivided before it is
    /// folded. Consider a list of numbers like `22 3 77 89 46`. If
    /// you used sequential fold to add them (`fold(0, |a,b| a+b)`,
    /// you would wind up first adding 0 + 22, then 22 + 3, then 25 +
    /// 77, and so forth. The **parallel fold** works similarly except
    /// that it first breaks up your list into sublists, and hence
    /// instead of yielding up a single sum at the end, it yields up
    /// multiple sums. The number of results is nondeterministic, as
    /// is the point where the breaks occur.
    ///
    /// So if did the same parallel fold (`fold(0, |a,b| a+b)`) on
    /// our example list, we might wind up with a sequence of two numbers,
    /// like so:
    ///
    /// ```notrust
    /// 22 3 77 89 46
    ///       |     |
    ///     102   135
    /// ```
    ///
    /// Or perhaps these three numbers:
    ///
    /// ```notrust
    /// 22 3 77 89 46
    ///       |  |  |
    ///     102 89 46
    /// ```
    ///
    /// In general, Rayon will attempt to find good breaking points
    /// that keep all of your cores busy.
    ///
    /// ### Fold versus reduce
    ///
    /// The `fold()` and `reduce()` methods each take an identity element
    /// and a combining function, but they operate rather differently.
    ///
    /// `reduce()` requires that the identity function has the same
    /// type as the things you are iterating over, and it fully
    /// reduces the list of items into a single item. So, for example,
    /// imagine we are iterating over a list of bytes `bytes: [128_u8,
    /// 64_u8, 64_u8]`. If we used `bytes.reduce(|| 0_u8, |a: u8, b:
    /// u8| a + b)`, we would get an overflow. This is because `0`,
    /// `a`, and `b` here are all bytes, just like the numbers in the
    /// list (I wrote the types explicitly above, but those are the
    /// only types you can use). To avoid the overflow, we would need
    /// to do something like `bytes.map(|b| b as u32).reduce(|| 0, |a,
    /// b| a + b)`, in which case our result would be `256`.
    ///
    /// In contrast, with `fold()`, the identity function does not
    /// have to have the same type as the things you are iterating
    /// over, and you potentially get back many results. So, if we
    /// continue with the `bytes` example from the previous paragraph,
    /// we could do `bytes.fold(|| 0_u32, |a, b| a + (b as u32))` to
    /// convert our bytes into `u32`. And of course we might not get
    /// back a single sum.
    ///
    /// There is a more subtle distinction as well, though it's
    /// actually implied by the above points. When you use `reduce()`,
    /// your reduction function is sometimes called with values that
    /// were never part of your original parallel iterator (for
    /// example, both the left and right might be a partial sum). With
    /// `fold()`, in contrast, the left value in the fold function is
    /// always the accumulator, and the right value is always from
    /// your original sequence.
    ///
    /// ### Fold vs Map/Reduce
    ///
    /// Fold makes sense if you have some operation where it is
    /// cheaper to groups of elements at a time. For example, imagine
    /// collecting characters into a string. If you were going to use
    /// map/reduce, you might try this:
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let s =
    ///     ['a', 'b', 'c', 'd', 'e']
    ///     .par_iter()
    ///     .map(|c: &char| format!("{}", c))
    ///     .reduce(|| String::new(),
    ///             |mut a: String, b: String| { a.push_str(&b); a });
    ///
    /// assert_eq!(s, "abcde");
    /// ```
    ///
    /// Because reduce produces the same type of element as its input,
    /// you have to first map each character into a string, and then
    /// you can reduce them. This means we create one string per
    /// element in ou iterator -- not so great. Using `fold`, we can
    /// do this instead:
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let s =
    ///     ['a', 'b', 'c', 'd', 'e']
    ///     .par_iter()
    ///     .fold(|| String::new(),
    ///             |mut s: String, c: &char| { s.push(*c); s })
    ///     .reduce(|| String::new(),
    ///             |mut a: String, b: String| { a.push_str(&b); a });
    ///
    /// assert_eq!(s, "abcde");
    /// ```
    ///
    /// Now `fold` will process groups of our characters at a time,
    /// and we only make one string per group. We should wind up with
    /// some small-ish number of strings roughly proportional to the
    /// number of CPUs you have (it will ultimately depend on how busy
    /// your processors are). Note that we still need to do a reduce
    /// afterwards to combine those groups of strings into a single
    /// string.
    ///
    /// You could use a similar trick to save partial results (e.g., a
    /// cache) or something similar.
    ///
    /// ### Combining fold with other operations
    ///
    /// You can combine `fold` with `reduce` if you want to produce a
    /// single value. This is then roughly equivalent to a map/reduce
    /// combination in effect:
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let bytes = 0..22_u8;
    /// let sum = bytes.into_par_iter()
    ///                .fold(|| 0_u32, |a: u32, b: u8| a + (b as u32))
    ///                .sum::<u32>();
    ///
    /// assert_eq!(sum, (0..22).sum()); // compare to sequential
    /// ```
    fn fold<T, ID, F>(self, identity: ID, fold_op: F) -> Fold<Self, ID, F>
        where F: Fn(T, Self::Item) -> T + Sync + Send,
              ID: Fn() -> T + Sync + Send,
              T: Send
    {
        fold::fold(self, identity, fold_op)
    }

    /// Applies `fold_op` to the given `init` value with each item of this
    /// iterator, finally producing the value for further use.
    ///
    /// This works essentially like `fold(|| init.clone(), fold_op)`, except
    /// it doesn't require the `init` type to be `Sync`, nor any other form
    /// of added synchronization.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let bytes = 0..22_u8;
    /// let sum = bytes.into_par_iter()
    ///                .fold_with(0_u32, |a: u32, b: u8| a + (b as u32))
    ///                .sum::<u32>();
    ///
    /// assert_eq!(sum, (0..22).sum()); // compare to sequential
    /// ```
    fn fold_with<F, T>(self, init: T, fold_op: F) -> FoldWith<Self, T, F>
        where F: Fn(T, Self::Item) -> T + Sync + Send,
              T: Send + Clone
    {
        fold::fold_with(self, init, fold_op)
    }

    /// Sums up the items in the iterator.
    ///
    /// Note that the order in items will be reduced is not specified,
    /// so if the `+` operator is not truly [associative] \(as is the
    /// case for floating point numbers), then the results are not
    /// fully deterministic.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    ///
    /// Basically equivalent to `self.reduce(|| 0, |a, b| a + b)`,
    /// except that the type of `0` and the `+` operation may vary
    /// depending on the type of value being produced.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 5, 7];
    ///
    /// let sum: i32 = a.par_iter().sum();
    ///
    /// assert_eq!(sum, 13);
    /// ```
    fn sum<S>(self) -> S
        where S: Send + Sum<Self::Item> + Sum<S>
    {
        sum::sum(self)
    }

    /// Multiplies all the items in the iterator.
    ///
    /// Note that the order in items will be reduced is not specified,
    /// so if the `*` operator is not truly [associative] \(as is the
    /// case for floating point numbers), then the results are not
    /// fully deterministic.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    ///
    /// Basically equivalent to `self.reduce(|| 1, |a, b| a * b)`,
    /// except that the type of `1` and the `*` operation may vary
    /// depending on the type of value being produced.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// fn factorial(n: u32) -> u32 {
    ///    (1..n+1).into_par_iter().product()
    /// }
    ///
    /// assert_eq!(factorial(0), 1);
    /// assert_eq!(factorial(1), 1);
    /// assert_eq!(factorial(5), 120);
    /// ```
    fn product<P>(self) -> P
        where P: Send + Product<Self::Item> + Product<P>
    {
        product::product(self)
    }

    /// Computes the minimum of all the items in the iterator. If the
    /// iterator is empty, `None` is returned; otherwise, `Some(min)`
    /// is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the `Ord` impl is not truly associative, then
    /// the results are not deterministic.
    ///
    /// Basically equivalent to `self.reduce_with(|a, b| cmp::min(a, b))`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [45, 74, 32];
    ///
    /// assert_eq!(a.par_iter().min(), Some(&32));
    ///
    /// let b: [i32; 0] = [];
    ///
    /// assert_eq!(b.par_iter().min(), None);
    /// ```
    fn min(self) -> Option<Self::Item>
        where Self::Item: Ord
    {
        self.reduce_with(cmp::min)
    }

    /// Computes the minimum of all the items in the iterator with respect to
    /// the given comparison function. If the iterator is empty, `None` is
    /// returned; otherwise, `Some(min)` is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the comparison function is not associative, then
    /// the results are not deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [-3_i32, 77, 53, 240, -1];
    ///
    /// assert_eq!(a.par_iter().min_by(|x, y| x.cmp(y)), Some(&-3));
    /// ```
    fn min_by<F>(self, f: F) -> Option<Self::Item>
        where F: Sync + Send + Fn(&Self::Item, &Self::Item) -> Ordering
    {
        self.reduce_with(|a, b| match f(&a, &b) {
                             Ordering::Greater => b,
                             _ => a,
                         })
    }

    /// Computes the item that yields the minimum value for the given
    /// function. If the iterator is empty, `None` is returned;
    /// otherwise, `Some(item)` is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the `Ord` impl is not truly associative, then
    /// the results are not deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [-3_i32, 34, 2, 5, -10, -3, -23];
    ///
    /// assert_eq!(a.par_iter().min_by_key(|x| x.abs()), Some(&2));
    /// ```
    fn min_by_key<K, F>(self, f: F) -> Option<Self::Item>
        where K: Ord + Send,
              F: Sync + Send + Fn(&Self::Item) -> K
    {
        self.map(|x| (f(&x), x))
            .min_by(|a, b| (a.0).cmp(&b.0))
            .map(|(_, x)| x)
    }

    /// Computes the maximum of all the items in the iterator. If the
    /// iterator is empty, `None` is returned; otherwise, `Some(max)`
    /// is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the `Ord` impl is not truly associative, then
    /// the results are not deterministic.
    ///
    /// Basically equivalent to `self.reduce_with(|a, b| cmp::max(a, b))`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [45, 74, 32];
    ///
    /// assert_eq!(a.par_iter().max(), Some(&74));
    ///
    /// let b: [i32; 0] = [];
    ///
    /// assert_eq!(b.par_iter().max(), None);
    /// ```
    fn max(self) -> Option<Self::Item>
        where Self::Item: Ord
    {
        self.reduce_with(cmp::max)
    }

    /// Computes the maximum of all the items in the iterator with respect to
    /// the given comparison function. If the iterator is empty, `None` is
    /// returned; otherwise, `Some(min)` is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the comparison function is not associative, then
    /// the results are not deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [-3_i32, 77, 53, 240, -1];
    ///
    /// assert_eq!(a.par_iter().max_by(|x, y| x.abs().cmp(&y.abs())), Some(&240));
    /// ```
    fn max_by<F>(self, f: F) -> Option<Self::Item>
        where F: Sync + Send + Fn(&Self::Item, &Self::Item) -> Ordering
    {
        self.reduce_with(|a, b| match f(&a, &b) {
                             Ordering::Greater => a,
                             _ => b,
                         })
    }

    /// Computes the item that yields the maximum value for the given
    /// function. If the iterator is empty, `None` is returned;
    /// otherwise, `Some(item)` is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the `Ord` impl is not truly associative, then
    /// the results are not deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [-3_i32, 34, 2, 5, -10, -3, -23];
    ///
    /// assert_eq!(a.par_iter().max_by_key(|x| x.abs()), Some(&34));
    /// ```
    fn max_by_key<K, F>(self, f: F) -> Option<Self::Item>
        where K: Ord + Send,
              F: Sync + Send + Fn(&Self::Item) -> K
    {
        self.map(|x| (f(&x), x))
            .max_by(|a, b| (a.0).cmp(&b.0))
            .map(|(_, x)| x)
    }

    /// Takes two iterators and creates a new iterator over both.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [0, 1, 2];
    /// let b = [9, 8, 7];
    ///
    /// let par_iter = a.par_iter().chain(b.par_iter());
    ///
    /// let chained: Vec<_> = par_iter.cloned().collect();
    ///
    /// assert_eq!(&chained[..], &[0, 1, 2, 9, 8, 7]);
    /// ```
    fn chain<C>(self, chain: C) -> Chain<Self, C::Iter>
        where C: IntoParallelIterator<Item = Self::Item>
    {
        chain::new(self, chain.into_par_iter())
    }

    /// Searches for **some** item in the parallel iterator that
    /// matches the given predicate and returns it. This operation
    /// is similar to [`find` on sequential iterators][find] but
    /// the item returned may not be the **first** one in the parallel
    /// sequence which matches, since we search the entire sequence in parallel.
    ///
    /// Once a match is found, we will attempt to stop processing
    /// the rest of the items in the iterator as soon as possible
    /// (just as `find` stops iterating once a match is found).
    ///
    /// [find]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.find
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3, 3];
    ///
    /// assert_eq!(a.par_iter().find_any(|&&x| x == 3), Some(&3));
    ///
    /// assert_eq!(a.par_iter().find_any(|&&x| x == 100), None);
    /// ```
    fn find_any<P>(self, predicate: P) -> Option<Self::Item>
        where P: Fn(&Self::Item) -> bool + Sync + Send
    {
        find::find(self, predicate)
    }

    /// Searches for the sequentially **first** item in the parallel iterator
    /// that matches the given predicate and returns it.
    ///
    /// Once a match is found, all attempts to the right of the match
    /// will be stopped, while attempts to the left must continue in case
    /// an earlier match is found.
    ///
    /// Note that not all parallel iterators have a useful order, much like
    /// sequential `HashMap` iteration, so "first" may be nebulous.  If you
    /// just want the first match that discovered anywhere in the iterator,
    /// `find_any` is a better choice.
    ///
    /// # Exmaples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3, 3];
    ///
    /// assert_eq!(a.par_iter().find_first(|&&x| x == 3), Some(&3));
    ///
    /// assert_eq!(a.par_iter().find_first(|&&x| x == 100), None);
    /// ```
    fn find_first<P>(self, predicate: P) -> Option<Self::Item>
        where P: Fn(&Self::Item) -> bool + Sync + Send
    {
        find_first_last::find_first(self, predicate)
    }

    /// Searches for the sequentially **last** item in the parallel iterator
    /// that matches the given predicate and returns it.
    ///
    /// Once a match is found, all attempts to the left of the match
    /// will be stopped, while attempts to the right must continue in case
    /// a later match is found.
    ///
    /// Note that not all parallel iterators have a useful order, much like
    /// sequential `HashMap` iteration, so "last" may be nebulous.  When the
    /// order doesn't actually matter to you, `find_any` is a better choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3, 3];
    ///
    /// assert_eq!(a.par_iter().find_last(|&&x| x == 3), Some(&3));
    ///
    /// assert_eq!(a.par_iter().find_last(|&&x| x == 100), None);
    /// ```
    fn find_last<P>(self, predicate: P) -> Option<Self::Item>
        where P: Fn(&Self::Item) -> bool + Sync + Send
    {
        find_first_last::find_last(self, predicate)
    }

    #[doc(hidden)]
    #[deprecated(note = "parallel `find` does not search in order -- use `find_any`, \\
    `find_first`, or `find_last`")]
    fn find<P>(self, predicate: P) -> Option<Self::Item>
        where P: Fn(&Self::Item) -> bool + Sync + Send
    {
        self.find_any(predicate)
    }

    /// Searches for **some** item in the parallel iterator that
    /// matches the given predicate, and if so returns true.  Once
    /// a match is found, we'll attempt to stop process the rest
    /// of the items.  Proving that there's no match, returning false,
    /// does require visiting every item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [0, 12, 3, 4, 0, 23, 0];
    ///
    /// let is_valid = a.par_iter().any(|&x| x > 10);
    ///
    /// assert!(is_valid);
    /// ```
    fn any<P>(self, predicate: P) -> bool
        where P: Fn(Self::Item) -> bool + Sync + Send
    {
        self.map(predicate).find_any(|&p| p).is_some()
    }

    /// Tests that every item in the parallel iterator matches the given
    /// predicate, and if so returns true.  If a counter-example is found,
    /// we'll attempt to stop processing more items, then return false.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [0, 12, 3, 4, 0, 23, 0];
    ///
    /// let is_valid = a.par_iter().all(|&x| x > 10);
    ///
    /// assert!(!is_valid);
    /// ```
    fn all<P>(self, predicate: P) -> bool
        where P: Fn(Self::Item) -> bool + Sync + Send
    {
        self.map(predicate).find_any(|&p| !p).is_none()
    }

    /// Creates an iterator over the `Some` items of this iterator, halting
    /// as soon as any `None` is found.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// let counter = AtomicUsize::new(0);
    /// let value = (0_i32..2048)
    ///     .into_par_iter()
    ///     .map(|x| {
    ///              counter.fetch_add(1, Ordering::SeqCst);
    ///              if x < 1024 { Some(x) } else { None }
    ///          })
    ///     .while_some()
    ///     .max();
    ///
    /// assert!(value < Some(1024));
    /// assert!(counter.load(Ordering::SeqCst) < 2048); // should not have visited every single one
    /// ```
    fn while_some<T>(self) -> WhileSome<Self>
        where Self: ParallelIterator<Item = Option<T>>,
              T: Send
    {
        while_some::new(self)
    }

    /// Create a fresh collection containing all the element produced
    /// by this parallel iterator.
    ///
    /// You may prefer to use `collect_into_vec()`, which allocates more
    /// efficiently with precise knowledge of how many elements the
    /// iterator contains, and even allows you to reuse an existing
    /// vector's backing store rather than allocating a fresh vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let sync_vec: Vec<_> = (0..100).into_iter().collect();
    ///
    /// let async_vec: Vec<_> = (0..100).into_par_iter().collect();
    ///
    /// assert_eq!(sync_vec, async_vec);
    /// ```
    fn collect<C>(self) -> C
        where C: FromParallelIterator<Self::Item>
    {
        C::from_par_iter(self)
    }

    /// Unzips the items of a parallel iterator into a pair of arbitrary
    /// `ParallelExtend` containers.
    ///
    /// You may prefer to use `unzip_into_vecs()`, which allocates more
    /// efficiently with precise knowledge of how many elements the
    /// iterator contains, and even allows you to reuse existing
    /// vectors' backing stores rather than allocating fresh vectors.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [(0, 1), (1, 2), (2, 3), (3, 4)];
    ///
    /// let (left, right): (Vec<_>, Vec<_>) = a.par_iter().cloned().unzip();
    ///
    /// assert_eq!(left, [0, 1, 2, 3]);
    /// assert_eq!(right, [1, 2, 3, 4]);
    /// ```
    fn unzip<A, B, FromA, FromB>(self) -> (FromA, FromB)
        where Self: ParallelIterator<Item = (A, B)>,
              FromA: Default + Send + ParallelExtend<A>,
              FromB: Default + Send + ParallelExtend<B>,
              A: Send,
              B: Send
    {
        unzip::unzip(self)
    }

    /// Partitions the items of a parallel iterator into a pair of arbitrary
    /// `ParallelExtend` containers.  Items for which the `predicate` returns
    /// true go into the first container, and the rest go into the second.
    ///
    /// Note: unlike the standard `Iterator::partition`, this allows distinct
    /// collection types for the left and right items.  This is more flexible,
    /// but may require new type annotations when converting sequential code
    /// that used type inferrence assuming the two were the same.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let (left, right): (Vec<_>, Vec<_>) = (0..8).into_par_iter().partition(|x| x % 2 == 0);
    ///
    /// assert_eq!(left, [0, 2, 4, 6]);
    /// assert_eq!(right, [1, 3, 5, 7]);
    /// ```
    fn partition<A, B, P>(self, predicate: P) -> (A, B)
        where A: Default + Send + ParallelExtend<Self::Item>,
              B: Default + Send + ParallelExtend<Self::Item>,
              P: Fn(&Self::Item) -> bool + Sync + Send
    {
        unzip::partition(self, predicate)
    }

    /// Partitions and maps the items of a parallel iterator into a pair of
    /// arbitrary `ParallelExtend` containers.  `Either::Left` items go into
    /// the first container, and `Either::Right` items go into the second.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use rayon::iter::Either;
    ///
    /// let (left, right): (Vec<_>, Vec<_>) = (0..8).into_par_iter()
    ///                                             .partition_map(|x| {
    ///                                                 if x % 2 == 0 {
    ///                                                     Either::Left(x * 4)
    ///                                                 } else {
    ///                                                     Either::Right(x * 3)
    ///                                                 }
    ///                                             });
    ///
    /// assert_eq!(left, [0, 8, 16, 24]);
    /// assert_eq!(right, [3, 9, 15, 21]);
    /// ```
    fn partition_map<A, B, P, L, R>(self, predicate: P) -> (A, B)
        where A: Default + Send + ParallelExtend<L>,
              B: Default + Send + ParallelExtend<R>,
              P: Fn(Self::Item) -> Either<L, R> + Sync + Send,
              L: Send,
              R: Send
    {
        unzip::partition_map(self, predicate)
    }

    /// Intersperses clones of an element between items of this iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let x = vec![1, 2, 3];
    /// let r: Vec<_> = x.into_par_iter().intersperse(-1).collect();
    ///
    /// assert_eq!(r, vec![1, -1, 2, -1, 3]);
    /// ```
    fn intersperse(self, element: Self::Item) -> Intersperse<Self>
        where Self::Item: Clone
    {
        intersperse::new(self, element)
    }

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method causes the iterator `self` to start producing
    /// items and to feed them to the consumer `consumer` one by one.
    /// It may split the consumer before doing so to create the
    /// opportunity to produce in parallel.
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: README.md
    fn drive_unindexed<C>(self, consumer: C) -> C::Result where C: UnindexedConsumer<Self::Item>;


    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// Returns the number of items produced by this iterator, if known
    /// statically. This can be used by consumers to trigger special fast
    /// paths. Therefore, if `Some(_)` is returned, this iterator must only
    /// use the (indexed) `Consumer` methods when driving a consumer, such
    /// as `split_at()`. Calling `UnindexedConsumer::split_off_left()` or
    /// other `UnindexedConsumer` methods -- or returning an inaccurate
    /// value -- may result in panics.
    ///
    /// This method is currently used to optimize `collect` for want
    /// of true Rust specialization; it may be removed when
    /// specialization is stable.
    fn opt_len(&self) -> Option<usize> {
        None
    }
}

impl<T: ParallelIterator> IntoParallelIterator for T {
    type Iter = T;
    type Item = T::Item;

    fn into_par_iter(self) -> T {
        self
    }
}

/// An iterator that supports "random access" to its data, meaning
/// that you can split it at arbitrary indices and draw data from
/// those points.
///
/// **Note:** Not implemented for `u64` and `i64` ranges
pub trait IndexedParallelIterator: ParallelIterator {
    /// Collects the results of the iterator into the specified
    /// vector. The vector is always truncated before execution
    /// begins. If possible, reusing the vector across calls can lead
    /// to better performance since it reuses the same backing buffer.
    fn collect_into_vec(self, target: &mut Vec<Self::Item>) {
        collect::collect_into_vec(self, target);
    }

    /// Unzips the results of the iterator into the specified
    /// vectors. The vectors are always truncated before execution
    /// begins. If possible, reusing the vectors across calls can lead
    /// to better performance since they reuse the same backing buffer.
    fn unzip_into_vecs<A, B>(self, left: &mut Vec<A>, right: &mut Vec<B>)
        where Self: IndexedParallelIterator<Item = (A, B)>,
              A: Send,
              B: Send
    {
        collect::unzip_into_vecs(self, left, right);
    }

    /// Iterate over tuples `(A, B)`, where the items `A` are from
    /// this iterator and `B` are from the iterator given as argument.
    /// Like the `zip` method on ordinary iterators, if the two
    /// iterators are of unequal length, you only get the items they
    /// have in common.
    fn zip<Z>(self, zip_op: Z) -> Zip<Self, Z::Iter>
        where Z: IntoParallelIterator,
              Z::Iter: IndexedParallelIterator
    {
        zip::new(self, zip_op.into_par_iter())
    }

    /// The same as `Zip`, but requires that both iterators have the same length.
    ///
    /// # Panics
    /// Will panic if `self` and `zip_op` are not the same length.
    ///
    /// ```should_panic
    /// use rayon::prelude::*;
    ///
    /// let one = [1u8];
    /// let two = [2u8, 2];
    /// let one_iter = one.par_iter();
    /// let two_iter = two.par_iter();
    ///
    /// // this will panic
    /// let zipped: Vec<(&u8, &u8)> = one_iter.zip_eq(two_iter).collect();
    ///
    /// // we should never get here
    /// assert_eq!(1, zipped.len());
    /// ```
    fn zip_eq<Z>(self, zip_op: Z) -> ZipEq<Self, Z::Iter>
        where Z: IntoParallelIterator,
              Z::Iter: IndexedParallelIterator
    {
        let zip_op_iter = zip_op.into_par_iter();
        assert_eq!(self.len(), zip_op_iter.len());
        zip_eq::new(self, zip_op_iter)
    }

    /// Interleave elements of this iterator and the other given
    /// iterator. Alternately yields elements from this iterator and
    /// the given iterator, until both are exhausted. If one iterator
    /// is exhausted before the other, the last elements are provided
    /// from the other.
    ///
    /// Example:
    ///
    /// ```
    /// use rayon::prelude::*;
    /// let (x, y) = (vec![1, 2], vec![3, 4, 5, 6]);
    /// let r: Vec<i32> = x.into_par_iter().interleave(y).collect();
    /// assert_eq!(r, vec![1, 3, 2, 4, 5, 6]);
    /// ```
    fn interleave<I>(self, other: I) -> Interleave<Self, I::Iter>
        where I: IntoParallelIterator<Item = Self::Item>,
              I::Iter: IndexedParallelIterator<Item = Self::Item>
    {
        interleave::new(self, other.into_par_iter())
    }

    /// Interleave elements of this iterator and the other given
    /// iterator, until one is exhausted.
    ///
    /// Example:
    ///
    /// ```
    /// use rayon::prelude::*;
    /// let (x, y) = (vec![1, 2, 3, 4], vec![5, 6]);
    /// let r: Vec<i32> = x.into_par_iter().interleave_shortest(y).collect();
    /// assert_eq!(r, vec![1, 5, 2, 6, 3]);
    /// ```
    fn interleave_shortest<I>(self, other: I) -> InterleaveShortest<Self, I::Iter>
        where I: IntoParallelIterator<Item = Self::Item>,
              I::Iter: IndexedParallelIterator<Item = Self::Item>
    {
        interleave_shortest::new(self, other.into_par_iter())
    }

    /// Split an iterator up into fixed-size chunks.
    ///
    /// Returns an iterator that returns `Vec`s of the given number of elements.
    /// If the number of elements in the iterator is not divisible by `chunk_size`,
    /// the last chunk may be shorter than `chunk_size`.
    ///
    /// See also [`par_chunks()`] and [`par_chunks_mut()`] for similar behavior on
    /// slices, without having to allocate intermediate `Vec`s for the chunks.
    ///
    /// [`par_chunks()`]: ../slice/trait.ParallelSlice.html#method.par_chunks
    /// [`par_chunks_mut()`]: ../slice/trait.ParallelSliceMut.html#method.par_chunks_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// let a = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    /// let r: Vec<Vec<i32>> = a.into_par_iter().chunks(3).collect();
    /// assert_eq!(r, vec![vec![1,2,3], vec![4,5,6], vec![7,8,9], vec![10]]);
    /// ```
    fn chunks(self, chunk_size: usize) -> Chunks<Self> {
        assert!(chunk_size != 0, "chunk_size must not be zero");
        chunks::new(self, chunk_size)
    }

    /// Lexicographically compares the elements of this `ParallelIterator` with those of
    /// another.
    fn cmp<I>(self, other: I) -> Ordering
        where I: IntoParallelIterator<Item = Self::Item>,
              I::Iter: IndexedParallelIterator,
              Self::Item: Ord
    {
        let other = other.into_par_iter();
        let ord_len = self.len().cmp(&other.len());
        self.zip(other)
            .map(|(x, y)| Ord::cmp(&x, &y))
            .find_first(|&ord| ord != Ordering::Equal)
            .unwrap_or(ord_len)
    }

    /// Lexicographically compares the elements of this `ParallelIterator` with those of
    /// another.
    fn partial_cmp<I>(self, other: I) -> Option<Ordering>
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              Self::Item: PartialOrd<I::Item>
    {
        let other = other.into_par_iter();
        let ord_len = self.len().cmp(&other.len());
        self.zip(other)
            .map(|(x, y)| PartialOrd::partial_cmp(&x, &y))
            .find_first(|&ord| ord != Some(Ordering::Equal))
            .unwrap_or(Some(ord_len))
    }

    /// Determines if the elements of this `ParallelIterator`
    /// are equal to those of another
    fn eq<I>(self, other: I) -> bool
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              Self::Item: PartialEq<I::Item>
    {
        let other = other.into_par_iter();
        self.len() == other.len() && self.zip(other).all(|(x, y)| x.eq(&y))
    }

    /// Determines if the elements of this `ParallelIterator`
    /// are unequal to those of another
    fn ne<I>(self, other: I) -> bool
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              Self::Item: PartialEq<I::Item>
    {
        !self.eq(other)
    }

    /// Determines if the elements of this `ParallelIterator`
    /// are lexicographically less than those of another.
    fn lt<I>(self, other: I) -> bool
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              Self::Item: PartialOrd<I::Item>
    {
        self.partial_cmp(other) == Some(Ordering::Less)
    }

    /// Determines if the elements of this `ParallelIterator`
    /// are less or equal to those of another.
    fn le<I>(self, other: I) -> bool
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              Self::Item: PartialOrd<I::Item>
    {
        let ord = self.partial_cmp(other);
        ord == Some(Ordering::Equal) || ord == Some(Ordering::Less)
    }

    /// Determines if the elements of this `ParallelIterator`
    /// are lexicographically greater than those of another.
    fn gt<I>(self, other: I) -> bool
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              Self::Item: PartialOrd<I::Item>
    {
        self.partial_cmp(other) == Some(Ordering::Greater)
    }

    /// Determines if the elements of this `ParallelIterator`
    /// are less or equal to those of another.
    fn ge<I>(self, other: I) -> bool
        where I: IntoParallelIterator,
              I::Iter: IndexedParallelIterator,
              Self::Item: PartialOrd<I::Item>
    {
        let ord = self.partial_cmp(other);
        ord == Some(Ordering::Equal) || ord == Some(Ordering::Greater)
    }

    /// Yields an index along with each item.
    fn enumerate(self) -> Enumerate<Self> {
        enumerate::new(self)
    }

    /// Creates an iterator that skips the first `n` elements.
    fn skip(self, n: usize) -> Skip<Self> {
        skip::new(self, n)
    }

    /// Creates an iterator that yields the first `n` elements.
    fn take(self, n: usize) -> Take<Self> {
        take::new(self, n)
    }

    /// Searches for **some** item in the parallel iterator that
    /// matches the given predicate, and returns its index.  Like
    /// `ParallelIterator::find_any`, the parallel search will not
    /// necessarily find the **first** match, and once a match is
    /// found we'll attempt to stop processing any more.
    fn position_any<P>(self, predicate: P) -> Option<usize>
        where P: Fn(Self::Item) -> bool + Sync + Send
    {
        self.map(predicate)
            .enumerate()
            .find_any(|&(_, p)| p)
            .map(|(i, _)| i)
    }

    /// Searches for the sequentially **first** item in the parallel iterator
    /// that matches the given predicate, and returns its index.
    ///
    /// Like `ParallelIterator::find_first`, once a match is found,
    /// all attempts to the right of the match will be stopped, while
    /// attempts to the left must continue in case an earlier match
    /// is found.
    ///
    /// Note that not all parallel iterators have a useful order, much like
    /// sequential `HashMap` iteration, so "first" may be nebulous.  If you
    /// just want the first match that discovered anywhere in the iterator,
    /// `position_any` is a better choice.
    fn position_first<P>(self, predicate: P) -> Option<usize>
        where P: Fn(Self::Item) -> bool + Sync + Send
    {
        self.map(predicate)
            .enumerate()
            .find_first(|&(_, p)| p)
            .map(|(i, _)| i)
    }

    /// Searches for the sequentially **last** item in the parallel iterator
    /// that matches the given predicate, and returns its index.
    ///
    /// Like `ParallelIterator::find_last`, once a match is found,
    /// all attempts to the left of the match will be stopped, while
    /// attempts to the right must continue in case a later match
    /// is found.
    ///
    /// Note that not all parallel iterators have a useful order, much like
    /// sequential `HashMap` iteration, so "last" may be nebulous.  When the
    /// order doesn't actually matter to you, `position_any` is a better
    /// choice.
    fn position_last<P>(self, predicate: P) -> Option<usize>
        where P: Fn(Self::Item) -> bool + Sync + Send
    {
        self.map(predicate)
            .enumerate()
            .find_last(|&(_, p)| p)
            .map(|(i, _)| i)
    }

    #[doc(hidden)]
    #[deprecated(note = "parallel `position` does not search in order -- use `position_any`, \\
    `position_first`, or `position_last`")]
    fn position<P>(self, predicate: P) -> Option<usize>
        where P: Fn(Self::Item) -> bool + Sync + Send
    {
        self.position_any(predicate)
    }

    /// Produces a new iterator with the elements of this iterator in
    /// reverse order.
    fn rev(self) -> Rev<Self> {
        rev::new(self)
    }

    /// Sets the minimum length of iterators desired to process in each
    /// thread.  Rayon will not split any smaller than this length, but
    /// of course an iterator could already be smaller to begin with.
    ///
    /// Producers like `zip` and `interleave` will use greater of the two
    /// minimums.
    /// Chained iterators and iterators inside `flat_map` may each use
    /// their own minimum length.
    fn with_min_len(self, min: usize) -> MinLen<Self> {
        len::new_min_len(self, min)
    }

    /// Sets the maximum length of iterators desired to process in each
    /// thread.  Rayon will try to split at least below this length,
    /// unless that would put it below the length from `with_min_len()`.
    /// For example, given min=10 and max=15, a length of 16 will not be
    /// split any further.
    ///
    /// Producers like `zip` and `interleave` will use lesser of the two
    /// maximums.
    /// Chained iterators and iterators inside `flat_map` may each use
    /// their own maximum length.
    fn with_max_len(self, max: usize) -> MaxLen<Self> {
        len::new_max_len(self, max)
    }

    /// Produces an exact count of how many items this iterator will
    /// produce, presuming no panic occurs.
    fn len(&self) -> usize;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method causes the iterator `self` to start producing
    /// items and to feed them to the consumer `consumer` one by one.
    /// It may split the consumer before doing so to create the
    /// opportunity to produce in parallel. If a split does happen, it
    /// will inform the consumer of the index where the split should
    /// occur (unlike `ParallelIterator::drive_unindexed()`).
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: README.md
    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method converts the iterator into a producer P and then
    /// invokes `callback.callback()` with P. Note that the type of
    /// this producer is not defined as part of the API, since
    /// `callback` must be defined generically for all producers. This
    /// allows the producer type to contain references; it also means
    /// that parallel iterators can adjust that type without causing a
    /// breaking change.
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: README.md
    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output;
}

/// `FromParallelIterator` implements the creation of a collection
/// from a [`ParallelIterator`]. By implementing
/// `FromParallelIterator` for a given type, you define how it will be
/// created from an iterator.
///
/// `FromParallelIterator` is used through [`ParallelIterator`]'s [`collect()`] method.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`collect()`]: trait.ParallelIterator.html#method.collect
///
/// # Examples
///
/// Implementing `FromParallelIterator` for your type:
///
/// ```
/// use rayon::prelude::*;
/// use std::mem;
///
/// struct BlackHole {
///     mass: usize,
/// }
///
/// impl<T: Send> FromParallelIterator<T> for BlackHole {
///     fn from_par_iter<I>(par_iter: I) -> Self
///         where I: IntoParallelIterator<Item = T>
///     {
///         let par_iter = par_iter.into_par_iter();
///         BlackHole {
///             mass: par_iter.count() * mem::size_of::<T>(),
///         }
///     }
/// }
///
/// let bh: BlackHole = (0i32..1000).into_par_iter().collect();
/// assert_eq!(bh.mass, 4000);
/// ```
pub trait FromParallelIterator<T>
    where T: Send
{
    /// Creates an instance of the collection from the parallel iterator `par_iter`.
    ///
    /// If your collection is not naturally parallel, the easiest (and
    /// fastest) way to do this is often to collect `par_iter` into a
    /// [`LinkedList`] or other intermediate data structure and then
    /// sequentially extend your collection. However, a more 'native'
    /// technique is to use the [`par_iter.fold`] or
    /// [`par_iter.fold_with`] methods to create the collection.
    /// Alternatively, if your collection is 'natively' parallel, you
    /// can use `par_iter.for_each` to process each element in turn.
    ///
    /// [`LinkedList`]: https://doc.rust-lang.org/std/collections/struct.LinkedList.html
    /// [`par_iter.fold`]: trait.ParallelIterator.html#method.fold
    /// [`par_iter.fold_with`]: trait.ParallelIterator.html#method.fold_with
    /// [`par_iter.for_each`]: trait.ParallelIterator.html#method.for_each
    fn from_par_iter<I>(par_iter: I) -> Self where I: IntoParallelIterator<Item = T>;
}

/// `ParallelExtend` extends an existing collection with items from a [`ParallelIterator`].
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
///
/// # Examples
///
/// Implementing `ParallelExtend` for your type:
///
/// ```
/// use rayon::prelude::*;
/// use std::mem;
///
/// struct BlackHole {
///     mass: usize,
/// }
///
/// impl<T: Send> ParallelExtend<T> for BlackHole {
///     fn par_extend<I>(&mut self, par_iter: I)
///         where I: IntoParallelIterator<Item = T>
///     {
///         let par_iter = par_iter.into_par_iter();
///         self.mass += par_iter.count() * mem::size_of::<T>();
///     }
/// }
///
/// let mut bh = BlackHole { mass: 0 };
/// bh.par_extend(0i32..1000);
/// assert_eq!(bh.mass, 4000);
/// bh.par_extend(0i64..10);
/// assert_eq!(bh.mass, 4080);
/// ```
pub trait ParallelExtend<T>
    where T: Send
{
    /// Extends an instance of the collection with the elements drawn
    /// from the parallel iterator `par_iter`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut vec = vec![];
    /// vec.par_extend(0..5);
    /// vec.par_extend((0..5).into_par_iter().map(|i| i * i));
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 0, 1, 4, 9, 16]);
    /// ```
    fn par_extend<I>(&mut self, par_iter: I) where I: IntoParallelIterator<Item = T>;
}
