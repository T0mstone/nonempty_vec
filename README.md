# nonempty_vec
This is a crate providing a single type, `NonEmptyVec`, which works just like `Vec` with a few differences:
  1. it is guaranteed to always have at least one element
  2. most* of its methods mirror those of `Vec` exactly

and two [traits](#tryfromiterator-and-trycollect)

*: as a result of 1., some methods take `NonZeroUsize` where the `Vec` counterpart would take `usize`

There is little to no documentation (because I'm lazy) but you can always look up a method's `Vec` counterpart to see its documentation.

### Missing `From` impls
`NonEmptyvec` does not provide any `From` impls, though you can always use `NonEmptyVec::from_vec(v)`

### `TryFromIterator` and `TryCollect`
As far as I know, such traits don't exist in `std` so I made them myself (since collecting an iterator to a `NonEmptyVec` is a neat thing to have).