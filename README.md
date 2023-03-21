# ඞ IMPOSTERS ඞ

Imposters is a rust library for creating and managing type erased item collections.

## Usage

To convert an item into an `Imposter`:

```rust
let item = MyStruct::new();
let imposter = Imposter::new(item);
```

Imposters can be downcast to retrieve the original items back out:

```rust
let original = imposter.downcast::<MyStruct>().unwrap();
```

> While imposters do erase the type and contain only a pointer to the original, the drop function for that type are stored as well. This means that if an `Imposeter` is dropped, it will correctly call the drop function for the underlying type as well.

Storing data in a type erased collection:

```rust
// Imposter vecs may be created when the struct type is known
let mut vec = ImposterVec::new::<MyStruct>();
vec.push_item(MyStruct::new());

// But also may be directly created from an imposter itself.
// This allows creation of new vecs without needing to know the containing type.
let imposter = Imposter::new(MyStruct::new());
let vec = ImposterVec::from_imposter(imposter);
```

> The `Imposter` struct has to store a bit of extra data long with the pointer to the original data like the `Layout`, `TypeId`, and drop function. However, when it is inserted into a vec, that information is scrubbed and the data is copied into a tightly packed array. This allows for incredibly fast iteration over the contained data without extra bloat.

## [MIT LICENSE](./LICENSE.txt)
