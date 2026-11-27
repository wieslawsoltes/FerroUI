# Changes for the upstream projects

FerroUI's patched copy of `skia-bindings` (`FERROUI-PATCH.md`) carries changes that belong in
Skia and in rust-skia. Each is here as a patch file whose first paragraphs are its description,
ready to be submitted by the owner of the repository. Nothing here is submitted automatically.

| File | Project | What it changes | Where it goes |
|---|---|---|---|
| `skia/0001-graphite-mtl-pipeline-references.patch` | Skia | `src/gpu/graphite/mtl/MtlGraphicsPipeline.mm`: the four objects a Metal render pipeline is created from are released after the descriptor retains them | A change on Skia's code review, against `main` (the patch is against the m153 milestone; the function is unchanged there) |
| `rust-skia/0001-graphite-vulkan-backend-texture.patch` | rust-skia | `skia-bindings/src/graphite.cpp`: `C_BackendTextures_MakeVulkan`, a Graphite backend texture of a VkImage | A pull request on rust-skia, against the release branch of 0.153 or its successor |

How the two relate to the copy in this repository:

- **The Metal ownership fix** is applied to the Skia source of a macOS Graphite build by
  `build_support/graphite_metal_lifetime.rs`, from `patches/graphite-metal-ownership.patch`, the
  same hunks. rust-skia builds Skia from its own fork of Skia, so after Skia takes the change,
  rust-skia's next Skia revision carries it, and a released `skia-bindings` with that revision
  lets FerroUI retire `external/rust-skia-bindings` for Metal.
- **The Vulkan backend texture** is in `src/graphite.cpp` of the copy once the Graphite Vulkan
  work (#1365) lands. The patch here is the same function with a comment for rust-skia. A safe
  wrapper in skia-safe is described in the patch and is not part of it.

Each patch is checked with `git apply --check -p1` against the file it names before it is
submitted: the Skia one against a Skia checkout at m153, the rust-skia one against the rust-skia
repository at 0.153.3.
