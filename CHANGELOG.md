# Changelog

## 0.20.0 (2026-04-12)

### Breaking Changes

- **config**: change default source_dir to empty string ([3e12971](https://github.com/urmzd/oag/commit/3e129719584f456f3027f2e9c82627e6b90be9ce))

### Documentation

- **config**: simplify default configuration example ([9c29330](https://github.com/urmzd/oag/commit/9c29330ac1d94755e1d18d22f57ff6b3316576c6))

### Miscellaneous

- sync Cargo.lock [skip ci] ([4908b6e](https://github.com/urmzd/oag/commit/4908b6ef653694641055aecc4643923f9689c87b))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.19.3...v0.20.0)


## 0.19.3 (2026-04-12)

### Documentation

- update architecture and CLI documentation ([c84046f](https://github.com/urmzd/oag/commit/c84046fc2a430098830c644d947b618f7c39a03e))

### Refactoring

- **cli**: simplify pack handling and argument flags ([9762d2c](https://github.com/urmzd/oag/commit/9762d2c7ad2c37c716b2451093842a79809e952c))
- **pack-resolution**: migrate packs to local .oag directory ([42b866d](https://github.com/urmzd/oag/commit/42b866d753787ec7997dfb2bf85eacac029a6a29))

### Miscellaneous

- sync Cargo.lock [skip ci] ([1d84e67](https://github.com/urmzd/oag/commit/1d84e6772851b3647817edb401c0609d55d013b5))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.19.2...v0.19.3)


## 0.19.2 (2026-04-09)

### Bug Fixes

- **ci**: remove --allow-dirty from cargo publish ([23deeb4](https://github.com/urmzd/oag/commit/23deeb4ce931ebeeb6a73c85ee02245f709ad045))

### Miscellaneous

- sync Cargo.lock [skip ci] ([9ac07b5](https://github.com/urmzd/oag/commit/9ac07b5e460b942f54455083c751238f969eb718))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.19.1...v0.19.2)


## 0.19.1 (2026-04-09)

### Bug Fixes

- **ci**: remove stale crate publishes and checkout release tag ([7efd31e](https://github.com/urmzd/oag/commit/7efd31ea3932c03060be0c102ac7149566aec729))

### Miscellaneous

- sync Cargo.lock [skip ci] ([76ae016](https://github.com/urmzd/oag/commit/76ae01613e6bb2a4c1b706bd14c4550ac6069882))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.19.0...v0.19.1)


## 0.19.0 (2026-04-09)

### Breaking Changes

- **inspect**: remove yaml output format support ([f4765de](https://github.com/urmzd/oag/commit/f4765de71a70188ca041b90cd748fe618f70f8fa))

### Features

- **cli**: add self-update and version subcommands ([a8ddd3f](https://github.com/urmzd/oag/commit/a8ddd3f784477afbe1fbd78aedf5cbb1e1d24446))

### Documentation

- add LICENSE to sub-crates for publishing compliance ([95bdc2c](https://github.com/urmzd/oag/commit/95bdc2c2d6215d534c4a8f7f884bc05cb2a39e91))

### Miscellaneous

- sync Cargo.lock [skip ci] ([4c522ee](https://github.com/urmzd/oag/commit/4c522ee8abe6d6f7c364e046cff1732fc7a283c3))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.18.0...v0.19.0)


## 0.18.0 (2026-04-08)

### Features

- **packs**: support extra_dev_dependencies in templates ([742520d](https://github.com/urmzd/oag/commit/742520df69ebb0fe1afba6ecbefc1d771adc186c))
- **cli**: add --force-scaffold flag to overwrite scaffold files ([cadd7ef](https://github.com/urmzd/oag/commit/cadd7efc6651e2c487b1d3e464892089233a3b1b))
- **core**: support write-once scaffold files and extra_dev_dependencies ([87a22d8](https://github.com/urmzd/oag/commit/87a22d8d1b5e77467dce98e48c3e836f272703f3))

### Documentation

- document force-scaffold and extra_dev_dependencies features ([6e79812](https://github.com/urmzd/oag/commit/6e798125eea27ab20709f5d7b7918559370b8d4c))

### Miscellaneous

- sync embedded files [skip ci] ([ce49b31](https://github.com/urmzd/oag/commit/ce49b3142a95b7e5f8e5a56ae4664bebb083c97e))
- add linguist overrides to fix language stats (#14) ([af0fd1c](https://github.com/urmzd/oag/commit/af0fd1cc4ce9c2530791f73101657b3bf8d191bc))
- sync Cargo.lock [skip ci] ([d17e6bb](https://github.com/urmzd/oag/commit/d17e6bbee07ecc188d45f762c489bf7a9cd381c1))
- **deps**: bump actions/create-github-app-token from 1 to 3 ([52f4286](https://github.com/urmzd/oag/commit/52f428664819c78965c8ceb5a8db1b04ea3f61bc))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.17.1...v0.18.0)


## 0.17.1 (2026-04-03)

### Bug Fixes

- **ci**: pull --rebase before push in Cargo.lock sync step (#13) ([f98c1f7](https://github.com/urmzd/oag/commit/f98c1f759d77af43d1a7bba9041a626c1f020898))

### Miscellaneous

- sync Cargo.lock [skip ci] ([1201e4c](https://github.com/urmzd/oag/commit/1201e4c48cec823dc8b4e4f071b376aac9763f2b))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.17.0...v0.17.1)


## 0.17.0 (2026-04-01)

### Features

- **packs**: add validator configurations to all packs ([0833fc2](https://github.com/urmzd/oag/commit/0833fc242445738c83cb848b66681f478d23d091))
- **cli**: add check command for validating generated output ([1a5a5ec](https://github.com/urmzd/oag/commit/1a5a5ecc167f38620dd7682bf431efdc2a771672))
- **core**: add validator configuration support to pack manifest ([5536e28](https://github.com/urmzd/oag/commit/5536e28ba5f213354489d6b2d32860bfd6647f94))

### Bug Fixes

- resolve release pipeline template failures (#11) ([aed893a](https://github.com/urmzd/oag/commit/aed893a0a9b2ef4770fb18dca9bae6b832ad34f4))

### Documentation

- update pack manifest filename references ([8a5e977](https://github.com/urmzd/oag/commit/8a5e977eb014556d4d007d1cc46f58314ae8aefb))

### Refactoring

- **templates**: fix RequestInit type in SSE stream handler ([5b58bbd](https://github.com/urmzd/oag/commit/5b58bbded0e961cd998092010d99d8e25ca5ac77))
- **cli**: support pinned pack versions and GitHub caching ([261b068](https://github.com/urmzd/oag/commit/261b0689c79e6dc979fe4aafba8c37ae258c47b7))
- **engine**: update pack resolution to use oag.pack.toml ([34f1fb7](https://github.com/urmzd/oag/commit/34f1fb79a972203ce518079017d5c45fd6184c09))
- **pack**: rename manifest files to oag.pack.toml ([9fabd36](https://github.com/urmzd/oag/commit/9fabd36d7dd8c97a3d83a01241fba736befaf55b))

### Miscellaneous

- **release**: add node setup and update pre-release validation ([7a5279b](https://github.com/urmzd/oag/commit/7a5279bede822c096b861c663e405350cc2632dd))
- **packs**: remove deprecated pack configurations ([26321fb](https://github.com/urmzd/oag/commit/26321fbf29f222de681069bbd7fca7459c10ef2c))
- **config**: add pre-release validation ([84103eb](https://github.com/urmzd/oag/commit/84103eb4b673dc1530c180055be6a70bfc94eba0))
- **build**: update cargo package name references ([208cb9c](https://github.com/urmzd/oag/commit/208cb9cb896449ea785c66706d0f7886ddbb5df0))
- **generated**: regenerate examples with guards and updated SSE ([e852169](https://github.com/urmzd/oag/commit/e852169679b1dcbf7103db958e72592b336357eb))
- sync Cargo.lock [skip ci] ([062e637](https://github.com/urmzd/oag/commit/062e6376c7cf95440aeafe17117bb3922ee3b01a))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.16.0...v0.17.0)


## 0.16.0 (2026-04-01)

### Breaking Changes

- **sse**: add full SSE spec support with event tracking ([35f5981](https://github.com/urmzd/oag/commit/35f59814f73e99cc4cb3f2b067066e244662de87))

### Miscellaneous

- **generated**: regenerate sse-chat react example ([50b3e4c](https://github.com/urmzd/oag/commit/50b3e4c39dd2924b0e8e2116b33c910975f4b3da))
- **generated**: regenerate sse-chat node example ([496c481](https://github.com/urmzd/oag/commit/496c48174d1cdae677a17a6f0432d354ae8c723f))
- **generated**: regenerate petstore example ([35fd289](https://github.com/urmzd/oag/commit/35fd289f8cc2db9c4081323098f95f87eedc3be0))
- **generated**: regenerate anthropic-messages react example ([7cbdd82](https://github.com/urmzd/oag/commit/7cbdd82c113901895207a7f5e7ed7a09717becfb))
- **generated**: regenerate anthropic-messages node example ([b0be1c6](https://github.com/urmzd/oag/commit/b0be1c6b672c046bf6037ce730485aaf79881c61))
- remove version pinning from teasr action ([4285d2f](https://github.com/urmzd/oag/commit/4285d2fd0f30aa23aa119e603db040f05c5eccee))
- upgrade node version from 22 to 24 (#10) ([e917fe8](https://github.com/urmzd/oag/commit/e917fe8d6cac3e80f7398197cd0565f5f5a239c3))
- update sr action from v2 to v3 ([f7dd005](https://github.com/urmzd/oag/commit/f7dd005539fbfa4a387c254ac4367d8c8ecbda86))
- sync Cargo.lock [skip ci] ([ec4ebe4](https://github.com/urmzd/oag/commit/ec4ebe4fb8782ea50e8e8bd6986386c031acfdc6))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.15.0...v0.16.0)


## 0.15.0 (2026-03-30)

### Features

- replace embedded packs with GitHub downloads ([bca6281](https://github.com/urmzd/oag/commit/bca6281d1bbf5acf4c817d6662d35e809c584080))

### Bug Fixes

- **demo**: update teasr demo for config-driven pack workflow ([112f6f3](https://github.com/urmzd/oag/commit/112f6f38e4f26af7aa3ff32109248eb453bc0d69))

### Miscellaneous

- sync Cargo.lock [skip ci] ([ffaf504](https://github.com/urmzd/oag/commit/ffaf504ce201cba96b956639898b34bd77a4d806))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.14.0...v0.15.0)


## 0.14.0 (2026-03-29)

### Features

- **packs**: create template pack manifests and migrate templates ([65ebc65](https://github.com/urmzd/oag/commit/65ebc65d01b7e47475fc6e24116bd57e95447863))
- **engine**: add template pack engine infrastructure ([ffd026b](https://github.com/urmzd/oag/commit/ffd026baa9df1db9dd3a6e4e6aaefc34123942fe))

### Bug Fixes

- create /tmp/oag-demo and add oag to PATH before teasr demo ([390f299](https://github.com/urmzd/oag/commit/390f29917bdf1b7dd952c841f5f4d9d7a4bb4578))
- use teasr action at repo root instead of nested path ([bf6439c](https://github.com/urmzd/oag/commit/bf6439c67c4cf2710075b21eca28f273242c98d7))
- **demo**: update teasr recording to match current config-driven CLI ([b94458e](https://github.com/urmzd/oag/commit/b94458e0c7428130b239cab07e1bd1995b271579))

### Documentation

- update documentation for template pack engine architecture ([5683019](https://github.com/urmzd/oag/commit/5683019f9411399e3ce24c66215c4f1c31381cba))
- update README ([e5be9b8](https://github.com/urmzd/oag/commit/e5be9b8fe58db22cc404488c6bec38d6af384fd7))
- **skills**: align SKILL.md with agentskills.io spec ([1376443](https://github.com/urmzd/oag/commit/1376443332d5550a3863f4f3a6a360659567dd5f))

### Refactoring

- **cli**: migrate to template pack system ([5545112](https://github.com/urmzd/oag/commit/55451127e12df428a2eeb7b272d778672f10547a))
- **config**: make generator id a string-based struct ([7f6888b](https://github.com/urmzd/oag/commit/7f6888b25db922354e59dea39ba70c5a8317a9f0))
- rename oag-cli package to oag for simpler cargo install ([453c3b5](https://github.com/urmzd/oag/commit/453c3b584b3df481d78c894b13ce07dd2316b2aa))

### Miscellaneous

- standardize sr.yaml — add refactor bump ([487b40e](https://github.com/urmzd/oag/commit/487b40e44c1258ce90acc564744b0fad280c5f14))
- **react-swr-client**: remove template files ([f8f6dc1](https://github.com/urmzd/oag/commit/f8f6dc12f494a8602c057664349afdf11464698d))
- **node-client**: remove template files ([83aac50](https://github.com/urmzd/oag/commit/83aac506b3368d845ba1bb225ff34679b06d5225))
- **fastapi-server**: remove template files ([da38a42](https://github.com/urmzd/oag/commit/da38a4270381e9e22805d01ad04ed28d4b401a1a))
- **build**: add semantic release hooks ([d44fa71](https://github.com/urmzd/oag/commit/d44fa7118027aec4550a187320c45540c717e262))
- **examples**: update generated petstore examples ([d9cd20f](https://github.com/urmzd/oag/commit/d9cd20fc35dbb80db6bfa62cb623744cefa59cf8))
- remove legacy generator crates ([b3fa669](https://github.com/urmzd/oag/commit/b3fa669e79b5b76c1fa99eafe5d90f0ae9056f6e))
- use sr-releaser GitHub App for release workflow (#6) ([73cf184](https://github.com/urmzd/oag/commit/73cf184296916706cce36c9c227eb7742db405cd))
- update semantic-release action to sr@v2 ([b23c1e6](https://github.com/urmzd/oag/commit/b23c1e6821bc659d8b0b5398cf04a22d8e77016e))
- **demo**: migrate recording from VHS to teasr ([1e132bc](https://github.com/urmzd/oag/commit/1e132bcb938aa6c20bf415f65341f844c71b3d2c))
- sync Cargo.lock [skip ci] ([0c8e532](https://github.com/urmzd/oag/commit/0c8e532e834f1e07568aa4e0603d3b8dbd77ab99))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.13.0...v0.14.0)


## 0.13.0 (2026-03-21)

### Features

- **cli**: add styled terminal output matching sr UI standard ([03298e9](https://github.com/urmzd/oag/commit/03298e950b4a7f9243201b8695b2ea09951c1a2c))

### Documentation

- add AGENTS.md and agent skill for Claude Code ([778e9f3](https://github.com/urmzd/oag/commit/778e9f33f0b2805d4e8630a6dc546d7835f7282b))

### Refactoring

- rename config to oag.yaml, remove toml/json support ([6a636c3](https://github.com/urmzd/oag/commit/6a636c3d932e8246b327a00a72960ad0d1053ca7))

### Miscellaneous

- rename repo from openapi-generator to oag, semantic-release to sr ([172ee40](https://github.com/urmzd/oag/commit/172ee40ffccff50c4ebd13793fe9802348b2b782))
- upgrade node version from 20 to 24 ([95075e3](https://github.com/urmzd/oag/commit/95075e395c2726ec35fa705b7c8b26febfb0f17c))
- standardize project files and README header ([fc3deb2](https://github.com/urmzd/oag/commit/fc3deb2c7480873d0c92b212bb32c917da354b0b))
- sync Cargo.lock [skip ci] ([d311eaf](https://github.com/urmzd/oag/commit/d311eaf4eea42d5bc8df7bf416d2b8129a3823a0))

[Full Changelog](https://github.com/urmzd/oag/compare/v0.12.0...v0.13.0)


## 0.12.0 (2026-03-09)

### Features

- support `scaffold: false` to disable scaffolding for existing projects ([de8637d](https://github.com/urmzd/openapi-generator/commit/de8637dd034414e27bc7a199a7f8f9a4c2ebfb8b))

### Documentation

- add comprehensive CLI reference, config comments, and schema documentation ([9aed1b8](https://github.com/urmzd/openapi-generator/commit/9aed1b89d3fb5d9b61ff586a00f1d884117e2aac))

### Miscellaneous

- switch to trusted publishing for crates.io ([9f301bf](https://github.com/urmzd/openapi-generator/commit/9f301bfaf3d8e07b5c9d0498c38cf400a0fa42a2))
- sync Cargo.lock [skip ci] ([8ac6954](https://github.com/urmzd/openapi-generator/commit/8ac69543908177a65a647ef5b72341ad1fbee40a))


## 0.11.1 (2026-02-25)

### Bug Fixes

- correct integer/union edge cases in const, discriminator guards, and Python enum base class ([7e4f414](https://github.com/urmzd/openapi-generator/commit/7e4f41434ce497d64f0b365fe11fb84a462cdb70))
- preserve integer enum values through IR translation ([b7f04d8](https://github.com/urmzd/openapi-generator/commit/b7f04d86bd83d1258f1d46f53b1398ef2c8c52dc))

### Miscellaneous

- fix clippy warnings in schema_resolver ([1c9098a](https://github.com/urmzd/openapi-generator/commit/1c9098a3422a5948e5eca5ae1913507217a97502))
- standardize GitHub Actions workflows ([47cba2a](https://github.com/urmzd/openapi-generator/commit/47cba2ac46666b3f6120c0df95cf5c2f87fefac2))
- sync Cargo.lock [skip ci] ([a5f003c](https://github.com/urmzd/openapi-generator/commit/a5f003c3ac66cefe7888fdf82e5126f0cc90955b))


## 0.11.0 (2026-02-25)

### Features

- add runtime type guards for discriminated unions ([da1dd8c](https://github.com/urmzd/openapi-generator/commit/da1dd8cf630e62f5603dc686d872246de495997a))

### Bug Fixes

- resolve clippy and rustfmt warnings in oag-node-client ([e04f03e](https://github.com/urmzd/openapi-generator/commit/e04f03e1a66ca1f34be6a64acb8bee0f7540f18c))
- trigger binary builds from release workflow ([e96e700](https://github.com/urmzd/openapi-generator/commit/e96e70020229d5baf910ad2ad7ba13478f0f8457))
- generate per-parameter query string serialization using OpenAPI style/explode ([77db2cf](https://github.com/urmzd/openapi-generator/commit/77db2cf85472eda712deb8917f2ce3e8b3df7948))

### Documentation

- remove License section from README ([2da9424](https://github.com/urmzd/openapi-generator/commit/2da9424295d1e1ed66e0eecaf78f01b845059d65))

### Miscellaneous

- move crates.io publish to separate job so build is never blocked ([58cd8cc](https://github.com/urmzd/openapi-generator/commit/58cd8cce67a0114217a203619773ba89e82ef5a0))
- inline build matrix into release.yml, remove build.yml ([bacafb1](https://github.com/urmzd/openapi-generator/commit/bacafb18c66a9221a0559590daf4fa6c71b828f6))
- float ([53ab565](https://github.com/urmzd/openapi-generator/commit/53ab56526f917f71958697a2e5de45ed91245213))
- update embed-src action to v3.1.0 ([d7182e7](https://github.com/urmzd/openapi-generator/commit/d7182e73eaa2d62d38159cafaa26cd9b764fde08))
- update embed-it references to embed-src ([c1777da](https://github.com/urmzd/openapi-generator/commit/c1777da94d36961d46eb0d2420684b68f459dfe6))
- split release and build workflows ([a0ff195](https://github.com/urmzd/openapi-generator/commit/a0ff195009bc8b177bb3c5ae0122494ef2d89453))
- add sensitive paths to .gitignore ([6f8ab4f](https://github.com/urmzd/openapi-generator/commit/6f8ab4f3077d449df931ca0155f1f6f04e21c9a2))
- sync Cargo.lock [skip ci] ([1639f73](https://github.com/urmzd/openapi-generator/commit/1639f73925664a7b430a7c71c49533ebd3e551e3))


## 0.10.0 (2026-02-21)

### Features

- add shell installer, Windows release target, and license fields ([0716e27](https://github.com/urmzd/openapi-generator/commit/0716e27e24ace9e2f162512aac02a90d20dd9552))

### Bug Fixes

- preserve $ref pointers to eliminate type duplication in generated output ([343bab7](https://github.com/urmzd/openapi-generator/commit/343bab75145c91782e35e05bd035c4ec17135cdb))

### Miscellaneous

- sync Cargo.lock [skip ci] ([4fea9f4](https://github.com/urmzd/openapi-generator/commit/4fea9f4a17c60690f4b099d0a7d3cc58a6eb1d2a))


## 0.9.0 (2026-02-17)

### Features

- use musl targets for static Linux binaries ([f4fbb14](https://github.com/urmzd/openapi-generator/commit/f4fbb1407e690959c7ef885aa61347e595531aa1))

### Miscellaneous

- sync Cargo.lock [skip ci] ([e036fff](https://github.com/urmzd/openapi-generator/commit/e036fff93d4f0f870dd2d19795be6f98d385888b))


## 0.8.0 (2026-02-14)

### Features

- add multipart uploads, retry with backoff, and ApiResponse wrapper ([d3f4a78](https://github.com/urmzd/openapi-generator/commit/d3f4a78d13cf2b3af84bf061a0512ebb9b02f7fe))

### Miscellaneous

- gitignore package-lock.json files in generated examples ([eb9d9fd](https://github.com/urmzd/openapi-generator/commit/eb9d9fd92a858b09bbed397fe918746b52342c0a))
- sync Cargo.lock [skip ci] ([b59f88b](https://github.com/urmzd/openapi-generator/commit/b59f88b3296b5e1718cfb1ee7b7ac3e0f9cf500d))


## 0.7.0 (2026-02-13)

### Features

- add pre-commit hook and fix formatting ([855bec1](https://github.com/urmzd/openapi-generator/commit/855bec1a6f4a51c350d32b5cd6ec4d6d74306431))

### Miscellaneous

- sync Cargo.lock [skip ci] ([a0eaa3b](https://github.com/urmzd/openapi-generator/commit/a0eaa3bfce7cfe2e18f15868b6f5458a6e2861eb))


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
