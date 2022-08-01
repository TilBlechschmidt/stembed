# Goals & Non-Goals

## Systems

**Goal:** Support english stenography\
**Non-goal:** Supporting any steno key layout out there

The initial goal is to support english stenography. Not more not less. We have explicitly ruled out support for any other system or layout. This decision may be revised in the future once the engine matures and the complexity afforded by allowing additional systems can be comprehended.

### English Stenography

**Goals:**
- Suffix folding
- Orthography

**Non-goals:**
- Prefix folding

## Dictionary formats

**Goal:** Plugin-driven dictionary formats\
**Non-goal:** Any specific format

At least on embedded, the engine will *always* use an optimized binary dictionary format. On desktop, support for any dictionary format may be implemented through plugins. The desktop application will be able to compile and upload all loaded dictionary entries onto an embedded device.

On both platforms, it will be possible to load plugins which provide computed dictionaries.

It is likely that a bare-bones plugin for importing a file format similar to Plover JSON will be provided. Though feature-parity should not be expected and some additional syntax may be introduced to cover differences between the Plover engine and this project.

## Plugins

### Loading

**Goal:** Plugins that alter the functionality\
**Non-goal:** Dynamic loading of plugins on embedded

Judging by the success of Plovers plugin ecosystem, plugins form the basis of a growing community around the steno engine. For this reason, we intend to develop an API which allows third-party (not authored by the project) code to alter the behaviour of this engine and provide functionality beyond the scope of this project.

However, since embedded platforms are very constrained in their resources, dynamically loading plugins and executing them via some interpretation will not be implemented. This decision is based on multiple conclusions:

- Running any kind of interpreted language — be it Lua, Python, JavaScript, Lisp, or rhai — embedded requires a huge amount of memory and a non-negligible amount of processing time. It can not be assumed that either of those is available in sufficient quantities.
- Using a language like Lua or JS on embedded would severely limit the capabilities of plugins on desktop as they do not have native access to e.g. the filesystem or network. This would either create a split ecosystem with plugins built for embedded and desktop only respectively, or restrict the kinds of plugins that would evolve.

All that being said, it should be made as simple as possible to build and install a custom firmware that contains additional plugins.

### Sandboxing

**Goal:** Sandboxed execution on desktop, optional access to system APIs

By default, plugins should not be able to access any APIs other than the provided one. This way, the potential attack surface is minimized. However, the user may optionally allow access to system APIs like the filesystem or network.

Currently, it is likely that this will be implemented through WASM and WASI.


### Cross-platform

**Goal:** Plugins that work both on embedded and desktop with the same code

When a developer builds a plugin that could work on both embedded and desktop (no allocations, no fs/net access), it should be able to do so without any modifications.

While the method of building & installing may be different (e.g. WASM on desktop, burned-in on embedded), the API surface used should be the same.
