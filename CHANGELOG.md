# Changelog

## 0.2.2 (2026-02-11)

### Bug Fixes

- move source files to src/, fix hook commas, add JSDoc escaping, and add existing_repo mode ([b9abe68](https://github.com/urmzd/openapi-generator/commit/b9abe6882ea5fae7040609e30c9a607182246a54))


## 0.2.1 (2026-02-11)

### Bug Fixes

- cross-validate and fix all documentation after crate rename ([12a9684](https://github.com/urmzd/openapi-generator/commit/12a96847d7f5bfcb2630cd88f6c0aac9d4ea78b0))
- update docs and CI configs to reflect crate renames and new scaffold schema ([b7ba88d](https://github.com/urmzd/openapi-generator/commit/b7ba88dbb886b61358f74d0a7b096acc1de352e3))


## 0.2.0 (2026-02-11)

### Features

- promote inline objects to named schemas for stronger type safety ([9704f53](https://github.com/urmzd/openapi-generator/commit/9704f535f9926618ff9b9607553eb456605a6aff))

### Bug Fixes

- apply cargo fmt to fix CI formatting check ([ae3e350](https://github.com/urmzd/openapi-generator/commit/ae3e35052d7ac19c8c75076cf61b63ca7e0b3a1b))

### Refactoring

- rename crates, add fastapi-server generator, and update core IR ([a29139a](https://github.com/urmzd/openapi-generator/commit/a29139ab47cd1df3040078bccad97b19a87e2b06))

### Miscellaneous

- update semantic-release action to v1 ([d23a5c6](https://github.com/urmzd/openapi-generator/commit/d23a5c6220c5af53c7322b3842fd6b00fbad8e22))
- update Cargo.toml license to Apache-2.0 ([1e96962](https://github.com/urmzd/openapi-generator/commit/1e969628b94459b0596d15a81f41f7ada0a5d7e7))
- license under Apache 2.0 ([70926f4](https://github.com/urmzd/openapi-generator/commit/70926f41f28677a776f02c04ab6f842bf8d92375))


## 0.1.1 (2026-02-11)

### Bug Fixes

- remove hooks, switch to semantic-release action ([a907524](https://github.com/urmzd/openapi-generator/commit/a9075240360bd5d20900d754a60e62cf94ce5709))


## 0.1.0 (2026-02-11)

### Features

- **cli**: add oag command-line interface ([4effe3b](https://github.com/urmzd/openapi-generator/commit/4effe3bb80cc8263824832aae20054815d78cb9c))
- **react**: add React/SWR hooks generator ([4768570](https://github.com/urmzd/openapi-generator/commit/476857078f5bd4f2c22f1bec6b30d14573676984))
- **typescript**: add TypeScript client code generator ([1292c8a](https://github.com/urmzd/openapi-generator/commit/1292c8a45e3bd829ab640a49a48afb03c42ecf64))
- **core**: add OpenAPI 3.2 parser, IR, and transforms ([4ec5d50](https://github.com/urmzd/openapi-generator/commit/4ec5d5019ee4fe3a02b936ffa4722caae53da4af))

### Documentation

- add colored splash screen and React/SSE demo to VHS recording ([b2429a3](https://github.com/urmzd/openapi-generator/commit/b2429a3ae5a7f634516117d5761b61ef31df572a))
- redesign demo recording with improved theme and layout ([e67fd97](https://github.com/urmzd/openapi-generator/commit/e67fd97317ed927794bcd805c24413a3808fcfb2))
- add crate-level READMEs for all workspace members ([3d08683](https://github.com/urmzd/openapi-generator/commit/3d08683b056adeb8dc9ad970c0700823dcf7d561))
- add CONTRIBUTING guide ([2d9942b](https://github.com/urmzd/openapi-generator/commit/2d9942bd3e63f27454a7c1245702adf2efb428b1))
- add root README with usage, philosophy, and architecture ([429965e](https://github.com/urmzd/openapi-generator/commit/429965e7835c977e8a8dfe73fd69a531615cf659))
- add petstore and sse-chat dogfooding examples ([5b00486](https://github.com/urmzd/openapi-generator/commit/5b0048657926983571c7b4cf9655e09ea29097c6))

### Miscellaneous

- fix VHS tarball extraction path ([1199f8b](https://github.com/urmzd/openapi-generator/commit/1199f8bcec8c47e93a04514f9bf9ea722f801ab7))
- install VHS to ~/.local/bin instead of /usr/local/bin ([3a91083](https://github.com/urmzd/openapi-generator/commit/3a910831a1e1d889b41619b73755263853916b4a))
- install VHS manually to work around vhs-action ffmpeg bug ([c3880c2](https://github.com/urmzd/openapi-generator/commit/c3880c24251ddb2752a99b06f2f537bbd703bcb2))
- use vhs-action instead of go install for VHS setup ([2fd68ce](https://github.com/urmzd/openapi-generator/commit/2fd68ce5aa075cbd693e13c51b4e564637984913))
- replace vhs setup action with manual installation ([8795c9c](https://github.com/urmzd/openapi-generator/commit/8795c9c35a5eaa1fb52d2003e7494bbab42b8583))
- add VHS demo recording to CI pipeline ([b139eaf](https://github.com/urmzd/openapi-generator/commit/b139eafa9ecd9a4dc5cd486f72482ea5d6d5171c))
- add pull request template ([a7e8e73](https://github.com/urmzd/openapi-generator/commit/a7e8e733349bc3aaef9926969a3579f9845cc2c0))
- add CI and release workflows with semantic-release and cargo publish ([2adc11a](https://github.com/urmzd/openapi-generator/commit/2adc11a28417ad306f1e82a88e72850c8a515386))
- add integration compile tests for TypeScript and React ([9c19ca9](https://github.com/urmzd/openapi-generator/commit/9c19ca949871e361579c8c76d9a3e21abe23a94d))
- initial project scaffold ([260c818](https://github.com/urmzd/openapi-generator/commit/260c818f4673e09f9556452a4ca0b63cb94bb8c6))
