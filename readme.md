### LightBlue Auto Message

A proof-of-concept protocol builder/collector built on [lightyear](https://github.com/cBournhonesque/lightyear)

---

A `lightyear` protocol requires that we collect all messages, components, and inputs into monolithic sum types. This is a common requirement, for example the types associated with an API endpoint must encompass all possible types it can accept or return. Instead of collecting these types manually, we can use macro markers to identify the types we want to encompass, and generate the sum types automatically during build.

For example, we can take the following lightyear `message_protocol` definition:
```rust
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message1(pub usize);

#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message2(pub usize);

#[message_protocol(protocol = "MyProtocol")]
pub enum Messages {
    Message1(Message1),
    Message2(Message2),
}
```

And reduce it to just the message types we want to handle:
```rust
#[message("MyProtocol")]
#[derive(Clone, Debug, PartialEq)]
pub struct Message1(pub usize);

#[message("MyProtocol")]
#[derive(Clone, Debug, PartialEq)]
pub struct Message2(pub usize);
```

You can then find the generated `MyProtocolMessages` in `crate::gen`

The same works for Components:
```rust
#[component(MyProtocol)]
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerColor(pub(crate) Color);
```

A skeleton is added to support input collection, but its implementation is left as an exercise to you. An alternative `#[inputs]` marker has been added for parity with `#[component_protocol]` and `#[message_protocol]`.

A `protocolize!` definition is finally generated using all items found, which in-turn generates the final protocol type. This example assumes the basic `protocolize!` structure given by the tutorial and simple boxes example, with Components, Messages, and Inputs. There are more parameters available in `protocolize!`, but they are not covered here for brevity.

---

This works by defining a macro in `lightblue_derive` that serves two purposes:
  - Adding implied derives
    - This example does not attempt to find derives already on the item, nor does it offer a way to opt-out, both good features to have in a true release
  - Marking structs for our parser

`build.rs` runs just before compiling our project. It walks the project, finds all relative items, and parses them. It generates the missing items and exports them to `OUT_DIR`, which becomes available to include when building the crate.

---

It's important to note that all types must be at least `pub(crate)`-accessible by their fully-qualified path at this time. This implementation is too dumb to understand re-exports, even at the module level - however it is certainly possible to accomplish with some elbow grease.

This crate currently assumes that all items necessary are defined in the same crate, which may not work for larger projects with shared definitions in separate crates. These can be supported by allowing one to define *partial* sum types that are erased by the macro and re-generated in `build`.

For brevity, this example only demonstrates building the protocol types, it does not use them as a more complete example might. The types generated here should be compatible with the [simple box example](https://github.com/cBournhonesque/lightyear/tree/0.11.0/examples/simple_box).
