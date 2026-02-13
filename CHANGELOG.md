# Changelog

## 0.6.1 (2026-02-13)

### Bug Fixes

- resolve 5 code generator bugs found in audit ([fe58045](https://github.com/urmzd/openapi-generator/commit/fe58045c2ac749b585a266fae815949d6b92dfbb))

### Miscellaneous

- fix rustfmt formatting in singularize function ([7cc03db](https://github.com/urmzd/openapi-generator/commit/7cc03dbf6dfd40ca08cac47652759caabeaf29c7))
- sync Cargo.lock [skip ci] ([1df9e88](https://github.com/urmzd/openapi-generator/commit/1df9e88e54b592af606cfad5a375b4330e4e5097))


## 0.6.0 (2026-02-13)

### Features

- add ApiError class with parsed body to generated clients ([55d5117](https://github.com/urmzd/openapi-generator/commit/55d5117dee75fe2ad691c0832aa39c0a7e29abb1))

### Miscellaneous

- regenerate petstore examples ([e444f5b](https://github.com/urmzd/openapi-generator/commit/e444f5bcb3c9d536ed4254364cc92e0d5f06929d))
- add SSE + query params compile tests for mixed-endpoints fixture ([19490d7](https://github.com/urmzd/openapi-generator/commit/19490d73f46dcce8ef3e20e5b1704190058888ae))
- sync Cargo.lock [skip ci] ([3cb5ba5](https://github.com/urmzd/openapi-generator/commit/3cb5ba5de00911bcaba81217cee2847bc384a081))


## 0.5.1 (2026-02-12)

### Bug Fixes

- **oag-react-swr-client**: properly format union type arrays in SSE hooks ([a535285](https://github.com/urmzd/openapi-generator/commit/a53528596ecfa74f9bf80cdb14734215b1718b9f))

### Miscellaneous

- sync Cargo.lock [skip ci] ([b14761a](https://github.com/urmzd/openapi-generator/commit/b14761a9dad505e53d81de492480ff565f85cb50))


## 0.5.0 (2026-02-12)

### Features

- add Anthropic Messages API fixture with advanced OpenAPI features ([cf0ac2f](https://github.com/urmzd/openapi-generator/commit/cf0ac2f94b23d4687ed4de0b2f31b2fc4f23e644))

### Bug Fixes

- use intersection types for mixed properties+additionalProperties, add petstore-polymorphic fixture, and run compile tests in CI ([e3d6f34](https://github.com/urmzd/openapi-generator/commit/e3d6f34c0c5a591f101ab025a0d3927ea15d7de1))
- auto-format parse_tests.rs ([d15c55b](https://github.com/urmzd/openapi-generator/commit/d15c55b02d9b41dd0ed50c430624ea8c94c1a158))

### Miscellaneous

- sync Cargo.lock [skip ci] ([327e2a2](https://github.com/urmzd/openapi-generator/commit/327e2a2596914c62f0ae38da16949139020f74cf))


## 0.4.3 (2026-02-12)

### Bug Fixes

- wire SSE query params, fix streaming hook params, and add discriminated union literals ([6ffc10a](https://github.com/urmzd/openapi-generator/commit/6ffc10a194d71b7854765ed6a2c33901c2a6ceb7))

### Miscellaneous

- sync Cargo.lock [skip ci] ([c5d39e4](https://github.com/urmzd/openapi-generator/commit/c5d39e4dde2354f6e595d33752c728f65eea09a4))


## 0.4.2 (2026-02-11)

### Bug Fixes

- auto-format code and add `just ci` recipe ([3613b31](https://github.com/urmzd/openapi-generator/commit/3613b31090867e5f268d0cd5eecf8b3c0076cbab))


## 0.4.1 (2026-02-11)

### Bug Fixes

- correct SWR mutation key types, fix SSE dedup, and add compile-check integration tests ([eae8680](https://github.com/urmzd/openapi-generator/commit/eae8680115caa44f4c7beb5b3a6b3c4ca42ab6d3))
- move default-config.yaml into oag-core crate for cargo publish ([eb0bfc1](https://github.com/urmzd/openapi-generator/commit/eb0bfc165955e1cc11d0d406d61151cc8c341238))


## 0.4.0 (2026-02-11)

### Features

- use embed-it to keep README config in sync with source ([92701db](https://github.com/urmzd/openapi-generator/commit/92701dbd537852d7124f0078682e50556ecf8420))

### Bug Fixes

- **ci**: chain embed-it before ci/build/release to prevent push race ([9ca4086](https://github.com/urmzd/openapi-generator/commit/9ca4086c462f152f9f6be9f0aea8010841ffa4a9))

### Documentation

- **source_dir**: document source_dir configuration option ([4ec8c46](https://github.com/urmzd/openapi-generator/commit/4ec8c46a3518eaa511321025648911efdf186dee))

### Miscellaneous

- auto-sync embedded files on push to main ([b01b88e](https://github.com/urmzd/openapi-generator/commit/b01b88e02a58520774698546fcbcb877b8a3555f))


## 0.3.0 (2026-02-11)

### Features

- make source_dir configurable on GeneratorConfig (default "src") ([1b185b3](https://github.com/urmzd/openapi-generator/commit/1b185b373f93a2781ecd65cff4e818dc28d298ae))


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
