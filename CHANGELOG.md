## [0.24.1](https://github.com/konvert7/klint/compare/v0.24.0...v0.24.1) (2026-06-07)


### Bug Fixes

* **skill:** extend with jsx-element docs ([54e0faa](https://github.com/konvert7/klint/commit/54e0faa12ab2887d3de4601073189796286ac4e0))

# [0.24.0](https://github.com/konvert7/klint/compare/v0.23.0...v0.24.0) (2026-05-27)


### Features

* add homebrew deployment ([c7aeee3](https://github.com/konvert7/klint/commit/c7aeee38c4480e3e606d9f3fc12da0a1afe62141))

# [0.23.0](https://github.com/konvert7/klint/compare/v0.22.0...v0.23.0) (2026-05-27)


### Features

* **package:** add --version flag and swift artifact creation in CI ([64b31d4](https://github.com/konvert7/klint/commit/64b31d49436b57fd27339d5c4993fb22acdd300d))
* **swift:** add support for Swift files in architecture rules ([6573193](https://github.com/konvert7/klint/commit/657319380260aa4fb6a151376d12dc6625af23c1))
* **swift:** enhance architecture rules to support Swift imports and module resolution ([fe058a7](https://github.com/konvert7/klint/commit/fe058a77a8d6ab00fa843e0a7b1509c566f389d3))

# [0.22.0](https://github.com/konvert7/klint/compare/v0.21.0...v0.22.0) (2026-05-27)


### Features

* **python:** make wheels version-agnostic ([65406de](https://github.com/konvert7/klint/commit/65406de24b288357dbad310c51c06e9bccc241d2))

# [0.21.0](https://github.com/konvert7/klint/compare/v0.20.0...v0.21.0) (2026-05-26)


### Features

* **python:** introduce klint Python package with wheel support and staging scripts ([1b58f6e](https://github.com/konvert7/klint/commit/1b58f6ef6f7d9c536607bd2ceac8492fbc2025d3))
* **rust:** add sonar/prefer-at rule for cleaner negative indexing ([738dd58](https://github.com/konvert7/klint/commit/738dd58eb362a44263b9977841426df2368b2adb))
* **rust:** add sonar/prefer-nullish-coalescing-assign rule for explicit nullish assignment handling ([2fd3f79](https://github.com/konvert7/klint/commit/2fd3f79f817231febe74d764b1f2eb6d89f73f1d))
* **rust:** add sonar/prefer-string-raw rule for handling escaped backslashes in string literals ([6376be3](https://github.com/konvert7/klint/commit/6376be328a6779762cb415d713a12dc2a72b3389))
* **rust:** add sonar/prefer-string-raw-regexp rule for improved RegExp template handling ([42e89a5](https://github.com/konvert7/klint/commit/42e89a5e0d219ebf4a5830bd209202ead15b8088))
* **rust:** add sonar/prefer-string-replaceall rule for improved string replacement ([f23a3c1](https://github.com/konvert7/klint/commit/f23a3c17646374024243c6fce7832eb6cccdb8f0))
* **rust:** add support for Python files in architecture rules ([65920c8](https://github.com/konvert7/klint/commit/65920c8147126672cd7838a8a6d401eaaf60fed9))
* **rust:** add support for sonar/no-single-char-class rule ([330e978](https://github.com/konvert7/klint/commit/330e9788edb769644db43f7790e5594c9ba4d303))
* **rust:** enhance architecture rules to resolve Python absolute imports ([7f5d503](https://github.com/konvert7/klint/commit/7f5d503251411eac850ee724c908a7f529fa4190))
* **rust:** enhance Rust engine to support built-in Sonar plugin defaults and reject unknown plugins ([b3d7d9f](https://github.com/konvert7/klint/commit/b3d7d9f62b8853249cc6014b50b337b229c4603d))
* **rust:** implement Python relative import handling in architecture rules ([f8ebd77](https://github.com/konvert7/klint/commit/f8ebd7712032abde4af23a996e3a92fecf1d5fb8))
* **rust:** restructure syntax module and add new rules for improved code analysis ([a3c0434](https://github.com/konvert7/klint/commit/a3c04343ad06344da1de76770cedb09ca77b1d66))

# [0.20.0](https://github.com/konvert7/klint/compare/v0.19.2...v0.20.0) (2026-05-26)


### Bug Fixes

* **dependencies:** update optional dependencies to use wildcard versioning ([c224b09](https://github.com/konvert7/klint/commit/c224b09f4b1fcde45fc7a675b77c3819c6e28710))
* **tests:** import readFileSync from node:fs in rust-engine-cli test ([9e95982](https://github.com/konvert7/klint/commit/9e95982f6f757805b7631cad058d37148a316ce1))
* **windows:** normalize paths ([3c585f9](https://github.com/konvert7/klint/commit/3c585f9f19481506bc077bf80b188c3e79854da5))


### Features

* **klint:** configure no-async-predicate ([f45f3c6](https://github.com/konvert7/klint/commit/f45f3c6c815150bd0b7cf60f5a43d88ae109cadd))
* **klint:** configure no-sync-in-async ([eaea7bf](https://github.com/konvert7/klint/commit/eaea7bf047d7a49137942ed6b80a7b0a0a3aa5c8))
* **klint:** configure sonar ([490587f](https://github.com/konvert7/klint/commit/490587f4dab50e9e2391d75b9a36feb8625369c8))
* **rust:** add 'no-sync-in-async' rule to detect synchronous calls in async functions ([efd0ecb](https://github.com/konvert7/klint/commit/efd0ecbb654948b50a9e9f6793c60d9cbac4e537))

## [0.19.2](https://github.com/konvert7/klint/compare/v0.19.1...v0.19.2) (2026-05-26)


### Bug Fixes

* **rust:** always pin native version to root version ([0ea534e](https://github.com/konvert7/klint/commit/0ea534e8c788e5b972b1f1ab9ca79d4be24de9a1))
* **tests:** mock readPackageJson to return a staged version for native binary resolution ([54b3dc5](https://github.com/konvert7/klint/commit/54b3dc547b6ffa52790d7a4ecb8f535b7e0fec45))

## [0.19.1](https://github.com/konvert7/klint/compare/v0.19.0...v0.19.1) (2026-05-26)


### Bug Fixes

* **rust:** enhance file resolution to support exclusion patterns and improve directory traversal ([8467dec](https://github.com/konvert7/klint/commit/8467dec1add0070aa272df6b1739435964675a70))

# [0.19.0](https://github.com/konvert7/klint/compare/v0.18.0...v0.19.0) (2026-05-26)


### Features

* **cli:** enhance Rust engine to support non-JSON output and improve error handling ([f4d9811](https://github.com/konvert7/klint/commit/f4d98115dc87cc1bd058041f2a2f5b749e71a487))
* **cli:** implement 'auto' engine for merging TypeScript and Rust rules ([2b0a867](https://github.com/konvert7/klint/commit/2b0a867fc77d9a38261d42231ba4a81a6e40cf59))

# [0.18.0](https://github.com/konvert7/klint/compare/v0.17.0...v0.18.0) (2026-05-26)


### Features

* **rules:** add no-consecutive-array-push rule to detect consecutive array push calls ([0bbaace](https://github.com/konvert7/klint/commit/0bbaace66fe7925a62349c0f0ad2cf9f5b74dc82))
* **rules:** add no-nested-template-literals rule with support for nested template literal detection ([c50bf60](https://github.com/konvert7/klint/commit/c50bf60d9961266c6406d8c99e7417c35d3e07de))
* **rules:** add no-unguarded-json-parse rule to detect unguarded JSON.parse calls ([99ca42a](https://github.com/konvert7/klint/commit/99ca42a2eaca08abfa464108e583e65252940f4a))

# [0.17.0](https://github.com/konvert7/klint/compare/v0.16.0...v0.17.0) (2026-05-26)


### Bug Fixes

* **rules:** normalize paths for no-string-match ([e267fbd](https://github.com/konvert7/klint/commit/e267fbd46bcb048d78a1156b5561c9c46a6a8551))


### Features

* **cli:** add support for engine selection in CLI and enhance error handling for unknown engines ([38cb989](https://github.com/konvert7/klint/commit/38cb989cf69025c6c7131a3c8a76648dd69d961d))
* introduce compare mode for safe rule support introduction in rust ([e4d72d3](https://github.com/konvert7/klint/commit/e4d72d3bd3e4d6380551785ec705d374c0467cee))
* **rust:** port no-string-match to rust ([4f118c8](https://github.com/konvert7/klint/commit/4f118c8ec4c23668ee2de4f50a6ad9ea2754b532))
* **tests:** add golden rule test cases for no-string-match rule ([4520789](https://github.com/konvert7/klint/commit/452078973394ed4b551006f572928303cad8877e))

# [0.16.0](https://github.com/konvert7/klint/compare/v0.15.2...v0.16.0) (2026-05-26)


### Bug Fixes

* **npm:** update package repo prefix ([3cbc9b4](https://github.com/konvert7/klint/commit/3cbc9b4c82318766254d8d72f461f85b8b6cbac2))


### Features

* **tests:** enhance tests for optional dependencies and CLI behavior ([1ffb37d](https://github.com/konvert7/klint/commit/1ffb37d61d988dc7a81d1525fe485b8b1237caec))

## [0.15.2](https://github.com/konvert7/klint/compare/v0.15.1...v0.15.2) (2026-05-26)


### Bug Fixes

* **npm:** add package metadata ([1df7908](https://github.com/konvert7/klint/commit/1df79087137182a85e9992afeebbe6fa5f0fdcb0))

## [0.15.1](https://github.com/konvert7/klint/compare/v0.15.0...v0.15.1) (2026-05-26)


### Bug Fixes

* **npm:** add option to skip missing packages ([6f059a0](https://github.com/konvert7/klint/commit/6f059a0ec1878fc4555664045b8ad0bd1488af89))

# [0.15.0](https://github.com/konvert7/klint/compare/v0.14.0...v0.15.0) (2026-05-26)


### Features

* **publish:** enhance native npm publishing with error handling and logging for missing packages ([7498d2e](https://github.com/konvert7/klint/commit/7498d2eaf7f23f2ee10fc9a65b70fdcae0abded9))

# [0.14.0](https://github.com/konvert7/klint/compare/v0.13.0...v0.14.0) (2026-05-26)


### Features

* **release:** implement native npm publishing and CI enhancements ([420c173](https://github.com/konvert7/klint/commit/420c1736f11aa377e34dee0e654fd0af858042a1))
* **tests:** add tests for resolving native package binaries for specific platforms ([2766f32](https://github.com/konvert7/klint/commit/2766f32db106c289e54f3065a50e48a6a0c623a8))

# [0.13.0](https://github.com/konvert7/klint/compare/v0.12.1...v0.13.0) (2026-05-25)


### Features

* **cli:** add execution time reporting to CLI output ([e015192](https://github.com/konvert7/klint/commit/e015192393be8ffb0cb278ec76b800cddb12da72))

## [0.12.1](https://github.com/konvert7/klint/compare/v0.12.0...v0.12.1) (2026-05-25)


### Bug Fixes

* **runner:** enhance directory exclusion logic and improve glob pattern matching ([5a29387](https://github.com/konvert7/klint/commit/5a29387d65663fe53eb61759283022466641ec68))

# [0.12.0](https://github.com/konvert7/klint/compare/v0.11.0...v0.12.0) (2026-05-25)


### Features

* **cli:** add debug mode for enhanced file resolution logging ([15d88a0](https://github.com/konvert7/klint/commit/15d88a084c1570a31b3e068678c0d005c02e9a2d))

# [0.11.0](https://github.com/konvert7/klint/compare/v0.10.0...v0.11.0) (2026-05-25)


### Features

* **release:** add dry-run script for native release preparation ([b798a3e](https://github.com/konvert7/klint/commit/b798a3eaada3888573823e9d91940e0843104477))
* **release:** implement custom npm release plugin and update CI configuration ([960960a](https://github.com/konvert7/klint/commit/960960a81ccccf09e9f44111ca86c902af896cf2))

# [0.10.0](https://github.com/konvert7/klint/compare/v0.9.0...v0.10.0) (2026-05-25)


### Features

* add Rust toolchain configuration and hooks for formatting, checking, and linting ([4f8ef90](https://github.com/konvert7/klint/commit/4f8ef901f2d6e1e6b75234148d2414317bfc84cf))
* **cli:** refactor Rust engine command resolution and enhance CLI error handling ([6f55885](https://github.com/konvert7/klint/commit/6f558853dd79e8e6c8e33d28bfe069ab27d82050))
* **hooks:** add pack check script to various configurations and CI workflow ([0547a55](https://github.com/konvert7/klint/commit/0547a55c1d2cbbd65d94814a0b5e5e1075611df2))
* implement architecture forbidden rules and file scanning ([81946cc](https://github.com/konvert7/klint/commit/81946cc3cfa3313891e89d9fd2f2769354df3ef1))
* initialize klint-rs crate with configuration handling and CLI ([0b4240d](https://github.com/konvert7/klint/commit/0b4240d96387335b6127dbe89705dc76c3f622d6))
* **native-binary:** add support for native binaries across platforms and implement resolution logic ([5f190d7](https://github.com/konvert7/klint/commit/5f190d7501963227c74869b87969ec3c26574217))
* **rust:** add JSX element scanning to architecture rules and enhance violation reporting ([109883e](https://github.com/konvert7/klint/commit/109883ec4d00a09b6551a6c34fa4b5a9e994cdf0))
* **rust:** add singleton rules to architecture configuration and enhance violation reporting ([04c9905](https://github.com/konvert7/klint/commit/04c9905469a19d0f788dee2b2089ca735b007cb7))
* **rust:** enhance import rules to allow type-only imports in deny mode and add corresponding tests ([54b4e3a](https://github.com/konvert7/klint/commit/54b4e3a03242060e6f143b21b72434f79ae362dc))
* **rust:** implement import rules for architecture configuration to manage import boundaries ([8814a18](https://github.com/konvert7/klint/commit/8814a184d3c3e9b3475a43e02b59e9640664ee89))
* **rust:** implement path alias resolution for TypeScript imports in architecture rules ([02a01a9](https://github.com/konvert7/klint/commit/02a01a921b6f2a1915a6b058bdd50c4b0c298152))
* **rust:** implement support for KLINT_ENGINE=rust with error handling and CLI integration ([7491640](https://github.com/konvert7/klint/commit/74916404cb75fa05f96a95fc1ded39938ab0dc1e))
* **rust:** integrate tree-sitter for TypeScript import scanning and add syntax module ([83934f0](https://github.com/konvert7/klint/commit/83934f0f22851f7ced74045627c337c99edf29d6))
* **rust:** update import rules to support allow mode with custom messages ([3ee72ba](https://github.com/konvert7/klint/commit/3ee72ba78ad782be0d30fc7058dfc57ed5a4be64))
* **tests:** add architecture rule test cases and golden fixtures for validation ([010aae8](https://github.com/konvert7/klint/commit/010aae8804ee2c2d3f440c77e91fc1303a73ed2c))
* **tests:** add native binary staging and smoke tests, update CI workflow ([e866ea7](https://github.com/konvert7/klint/commit/e866ea71b88b7a46c6674b0b3a865be518849ddc))
* **tests:** add pack check script and corresponding test for package integrity ([071d7b4](https://github.com/konvert7/klint/commit/071d7b44fe54aa1f84d062f91140f2d9bfafaaf4))
* **tests:** add rust-engine test command and integrate into CI and hooks ([c765f50](https://github.com/konvert7/klint/commit/c765f50e03105dd09beec58c1bb6407f85d3e8e8))
* **tests:** enhance architecture rule tests with Rust support and update violation cases ([bd42ab1](https://github.com/konvert7/klint/commit/bd42ab18194d8d6a121f3f60e5a0857f8c375eda))

# [0.9.0](https://github.com/konvert7/klint/compare/v0.8.0...v0.9.0) (2026-05-25)


### Features

* enhance jscpd hook with format handling and add hooks configuration ([e637ec6](https://github.com/konvert7/klint/commit/e637ec6efe2dd0ca908717e7dc6d0b3e7ecbc7b5))

# [0.8.0](https://github.com/konvert7/klint/compare/v0.7.1...v0.8.0) (2026-05-21)


### Features

* add jsx element matching ([fc63562](https://github.com/konvert7/klint/commit/fc63562460939eec01b063e98e5072cc0f9c3ce0))

## [0.7.1](https://github.com/konvert7/klint/compare/v0.7.0...v0.7.1) (2026-05-20)


### Bug Fixes

* respect glob ignore patterns ([594e243](https://github.com/konvert7/klint/commit/594e2433a53afa8e39f98433a661688517b75ce8))

# [0.7.0](https://github.com/konvert7/klint/compare/v0.6.0...v0.7.0) (2026-05-20)


### Features

* **sonar:** add meta to schema ([dbdb45f](https://github.com/konvert7/klint/commit/dbdb45f781579bf8af0e0328a0c9458eeaecb014))

# [0.6.0](https://github.com/konvert7/klint/compare/v0.5.0...v0.6.0) (2026-05-19)


### Features

* add jscpd to pre-commit hook ([16b202f](https://github.com/konvert7/klint/commit/16b202fc9115c5242a226e6ed511ae57a1a1768a))
* implement rule helpers for AST-based rule definitions ([5670607](https://github.com/konvert7/klint/commit/56706072469a834fdb3d785f73aaecd74bc5ba8d))
* integrate jscpd ([692a3ab](https://github.com/konvert7/klint/commit/692a3ab7aed83ee99dfe10830299e5c0a9cd56c9))

# [0.5.0](https://github.com/konvert7/klint/compare/v0.4.0...v0.5.0) (2026-05-19)


### Features

* extend schema ([a042e74](https://github.com/konvert7/klint/commit/a042e744ffce15903ad9992e33e4b538d9e3d3af))

# [0.4.0](https://github.com/konvert7/klint/compare/v0.3.0...v0.4.0) (2026-05-18)


### Features

* generate schema ([a5a72dd](https://github.com/konvert7/klint/commit/a5a72dd344ef5d9693bb4365fce95d8f131ce4d6))

# [0.3.0](https://github.com/konvert7/klint/compare/v0.2.0...v0.3.0) (2026-05-17)


### Features

* add agent hooks ([dedafc4](https://github.com/konvert7/klint/commit/dedafc4338701d0e0f203fce3aa7a7305800ac33))
* add help and h to open help page next to --help ([ffff2e2](https://github.com/konvert7/klint/commit/ffff2e206d356c53999d18185e73d11d31f02907))
* add klint rules for the repo itself ([3bf5061](https://github.com/konvert7/klint/commit/3bf50615b4d0f19456f5743f90fe9b068434af58))
* install klint skill ([1083ffc](https://github.com/konvert7/klint/commit/1083ffc5d911bf605de108019dfd17a06e3283a3))

# [0.2.0](https://github.com/konvert7/klint/compare/v0.1.0...v0.2.0) (2026-05-17)


### Bug Fixes

* detect correct platform-independent absolute path ([6afe6f6](https://github.com/konvert7/klint/commit/6afe6f63694c82c33d2b273b07c137ecee77a952))
* normalize windows paths ([3aa72b0](https://github.com/konvert7/klint/commit/3aa72b0a449cf9561fba68014adbd5e50d7c0020))


### Features

* initial klint package ([3747166](https://github.com/konvert7/klint/commit/3747166bd1ca1a7b215cd409d7d6f83ebe84a43c))
