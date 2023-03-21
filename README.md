# ඞ IMPOSTERS ඞ

[![Rust](https://github.com/rhedgeco/imposters/actions/workflows/rust.yml/badge.svg)](https://github.com/rhedgeco/imposters/actions/workflows/rust.yml)

Imposters is a rust library for creating and managing type erased item collections.

## Why not use `Box<dyn Any>` or `Vec<Box<dyn Any>>`?

While using `Box<dyn Any>` may achieve the same outcome in terms of state, it is severely lacking in speed and efficiency. A `Box` allocates space for its own memory, so what you end up getting when using `Vec<Box<dyn Any>>` is **multiple** levels of pointer indirection. Not only does this come at the cost of incredibly bad cache efficiency, but even if we forgot about that, having to move down 3 different pointers is pretty overkill for whats trying to be achieved.

### ***What do I mean by pointer indirection?***

Well if we take a struct and wrap it in a `Box<dyn Any>`. We get our first two levels of indirection and unique allocation. When we convert our struct to a `dyn Any`, it converts it into whats called a *fat-pointer*. The trait holds a pointer to the location of the data, and a pointer to the data itself. Not only that but then we wrap it in a `Box`. The box then allocates memory on the heap, and stores these two pointers there. Then the box stores the location of that data as a pointer. *We are now two pointers deep just to create the boxed any*. NOT ONLY THAT, but now if we create a `Vec` to store multiple of these, the vec allocates its own memory on the heap and stores the boxes in there!

So to get the original data back, we have to index into the `Vec`, then take the pointer in the box to the location of the `dyn Any`, then take the pointer in the `Any` to the location of the actual data. WOW that was roundabout. AND it wrecks our cache efficiency too!

### ***Whats all this about cache efficiency?***

Modern computers have multiple places that data is stored, and typically the closer to the CPU that data is, the faster it is to access. Obviously the worst place for the data to be is on another computer. Not as bad but still slow is on the computers hard disk. Then we get into RAM. Ram is definitely fast, but there is *much* faster. Right next to the CPU are generally L1 and L2 cache. L1 and L2 are *BLAZINGLY FAST*.

When data from a certain location is accessed, first the CPU checks the L1 cache, if it isn't there it checks L2, and if it isn't there it loads the location from RAM. But it also loads the data around it as well. This is because its often good practice to keep all the data you need for something next to each other. So when you load the next piece of data, it might already be in one of those cache levels.

But as we talked about, when accessing all this data in the `Vec<Box<dyn Any>>`. All this gets thrown out the window as the data is stored in numerous different places in RAM. So when accessing an item in the vec, we can be almost certain that the data will never be in the L1 or L2 cache. This is whats called a **cache-miss** and many try their absolute hardest to avoid them in performance critical contexts.

### ***So what does the `imposters` library do different?***

While the `Imposter` struct is very similar to a `Box<dyn Any>` as it holds a pointer to the data and a pointer to the drop function, among some other metadata. The `ImposterVec` (and potentially future collections) is where this library really shines.

When inserting data or and `Imposter` into an `ImposterVec`, all layers of indirection are scrubbed. The imposter source data is copied into the array inside the vec. This means that the data that needs to be accessed sits right next its other data in an as tightly packed configuration as possible. Thus, there is only 1 pointer involved that points to the start of the array with no other indirection. And because of this, when iterating over the vec, as much of the memory will be **cache-hits** as often as possible keeping your code ***Blazingly Fast™***.

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

> While imposters do erase the type and contain only a pointer to the original, the drop function for that type are stored as well. This means that if an `Imposter` is dropped, it will correctly call the drop function for the underlying type as well.

Storing data in a type erased collection:

```rust
// ImposterVec may be created when the struct type is known
let mut vec = ImposterVec::new::<MyStruct>();
vec.push_item(MyStruct::new());

// But also may be directly created from an imposter itself.
// This allows creation of new vectors without needing to know the containing type.
let imposter = Imposter::new(MyStruct::new());
let vec = ImposterVec::from_imposter(imposter);
```

> The `Imposter` struct has to store a bit of extra data along with the pointer to the original data like the `Layout`, `TypeId`, and drop function. However, when it is inserted into a vec, that information is scrubbed and the data is copied into a tightly packed array. This allows for incredibly fast iteration over the contained data without extra bloat.

## [MIT LICENSE](./LICENSE.txt)
