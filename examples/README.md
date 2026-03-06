# Apex Examples

This folder is the executable language feature gallery.

## Core language

- `01_hello.apex`: IO basics, strings.
- `02_variables.apex`: variable declarations, mutability, primitive types.
- `03_math.apex`: arithmetic, comparisons, math stdlib.
- `04_control_flow.apex`: `if`, `while`, `for`, `break`, `continue`.
- `05_classes.apex`: classes, constructors, destructors, methods, visibility.
- `06_enums.apex`: enums and variant modeling basics.
- `07_interfaces.apex`: interface declaration patterns and class integration examples.
- `08_modules.apex`: modules and namespaced calls.
- `09_generics.apex`: `Option`, `Result`, `List`, `Map`, `Set`, smart pointers.
- `10_ownership.apex`: ownership and borrowing examples.
- `11_lambdas.apex`: lambdas, closures, higher-order functions.
- `12_string_interp.apex`: string interpolation patterns.
- `13_error_handling.apex`: `Option`, `Result`, `?`, `require`.
- `14_async.apex`: async/await and async blocks.
- `15_stdlib.apex`: stdlib overview usage.
- `16_pattern_matching.apex`: pattern matching flows.
- `17_comprehensive.apex`: mixed end-to-end language usage.
- `18_file_io.apex`: file operations.
- `19_time.apex`: time functions.
- `20_system.apex`: system calls/utilities.
- `21_conversions.apex`: type conversions.
- `22_args.apex`: command-line args.
- `23_str_utils.apex`: string utilities.
- `24_test_attributes.apex`: test attributes and assertions.
- `25_range_types.apex`: `Range<T>` and iterators.
- `26_effect_system.apex`: effect attributes (`@Pure`, `@Io`, `@Thread`).
- `27_extern_c_interop.apex`: `extern function` C interop.
- `28_async_runtime_control.apex`: true async runtime task controls (`is_done`, `await_timeout`, `cancel`).
- `29_effect_inference_and_any.apex`: automatic effect inference and `@Any`.
- `30_extern_variadic_printf.apex`: variadic `extern` calls (`printf` with `...`).
- `31_extern_abi_link_name.apex`: explicit ABI + symbol aliasing (`extern(c, "symbol")`).
- `32_extern_safe_wrapper.apex`: safe Apex wrapper around raw extern call.
- `33_extern_ptr_types.apex`: `Ptr<T>` extern signatures (`malloc`/`free` style APIs).
- `34_bindgen_workflow.apex`: usage style for generated `apex bindgen` extern declarations.
- `35_visibility_enforcement.apex`: enforced `public`/`private`/`protected` access rules.
- `36_inheritance_extends.apex`: class inheritance with `extends` and inherited methods.
- `37_interfaces_contracts.apex`: enforced `implements` contracts and interface-typed params.
- `38_import_aliases.apex`: `import ... as ...` alias usage (`math`, `str`, `io`).
- `39_compound_assign.apex`: compound assignment operators (`+=`, `-=`, `*=`, `/=`) for vars, indexes, and fields.
- `40_borrow_scope_recovery.apex`: borrow checker scope edge case where immutable borrow ends and move is valid again.

## Full apps

- `app_bank.apex`
- `app_calculator.apex`
- `app_data_structures.apex`
- `app_game.apex`
- `app_notes.apex`
- `app_todo.apex`

## Multi-file projects

- `multi_file_project/`
- `multi_file_depth_project/`
- `test_no_import/`

## Run all examples

On Linux:
```bash
./scripts/test_examples_linux.sh
```

On Windows
```bash
./scripts/test_examples.bat
```

Mac OS
```bash
./scripts/test_examples_macos.sh
```
