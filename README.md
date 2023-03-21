# ඞ IMPOSTERS ඞ

[![Rust](https://github.com/rhedgeco/imposters/actions/workflows/rust.yml/badge.svg)](https://github.com/rhedgeco/imposters/actions/workflows/rust.yml)

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

## Why not use `Box<dyn Any>` or `Vec<Box<dyn Any>>`?

While using `Box<dyn Any>` may achieve the same outcome in terms of state, it is severely lacking in speed and efficiency. A `Box` allocates space for its own memory, so what you end up getting when using `Vec<Box<dyn Any>>` is **multiple** levels of pointer indirection. And this comes at the cost of really bad cache efficiency.

### ***What do I mean by pointer indirection?***

Well if we take a struct and wrap it in a `Box<dyn Any>`. We get our first level of indirection and unique allocation. When we convert our struct to a `dyn Any` and box it up, it converts it into what's called a *fat-pointer*. The boxed trait holds a pointer to the location of the data, and a pointer to the vtable for the trait. Not only that but the box allocates memory on the heap and stores these two pointers there. NOT ONLY THAT but now if we create a `Vec` to store multiple of these, the vector allocates its own memory on the heap and stores the boxes in there!

So to get the original data back, we have to index into the `Vec`, and then take the pointer in the box to the *ACTUAL* location of data, which is quite roundabout. And it wrecks our cache efficiency too!

### ***What's all this about cache efficiency?***

Modern computers have multiple places where data is stored, and typically the closest to the CPU that data is, the faster it is to access. Obviously, the worst place for the data to be is on another computer. Not as bad but still slow is the computer's hard disk. Then we get into RAM. Ram is definitely fast, but there is *much* faster. Right next to the CPU are generally L1 and L2 caches. L1 and L2 are *BLAZINGLY FAST*.

When data from a certain location is accessed, first the CPU checks the L1 cache, if it isn't there it checks L2, and if it isn't there it loads the location from RAM. But it also loads the data around it as well. This is because it's often good practice to keep all the data you need for something next to each other. So when you load the next piece of data, it might already be in one of those cache levels.

But as we talked about when accessing all this data in the `Vec<Box<dyn Any>>`, all this gets thrown out the window as the data is stored in numerous different places in RAM. So when accessing an item in the vector, we can be almost certain that the data will never be in the L1 or L2 cache. This is what's called a **cache-miss** and many try their absolute hardest to avoid them in performance-critical contexts.

### ***So what does the `imposters` library do differently?***

While the `Imposter` struct is very similar to a `Box<dyn Any>` as it holds a pointer to the data and a pointer to the drop function, among some other metadata. The `ImposterVec` (and potentially future collections) is where this library really shines.

When inserting data or an `Imposter` into an `ImposterVec`, all layers of indirection are scrubbed. The imposter source data is copied into the array inside the vector. This means that the data that needs to be accessed sits right next to its other data in a tightly packed configuration. Thus, there is only 1 pointer involved that points to the start of the array with no other indirection. And because of this, when iterating over the vector, as much of the memory will be **cache-hits** as often as possible keeping your code ***Blazingly Fast™***.

## [MIT LICENSE](./LICENSE.txt)
