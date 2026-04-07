# Arden Examples

This folder is the executable language feature gallery.

## Core language

- `01_hello.arden`: IO basics, strings.
- `02_variables.arden`: variable declarations, mutability, primitive types.
- `03_math.arden`: arithmetic, comparisons, math stdlib.
- `04_control_flow.arden`: `if`, `while`, `for`, `break`, `continue`.
- `05_classes.arden`: classes, constructors, destructors, methods, visibility.
- `06_enums.arden`: enums and variant modeling basics.
- `07_interfaces.arden`: interface declaration patterns and class integration examples.
- `08_modules.arden`: modules and namespaced calls.
- `09_generics.arden`: `Option`, `Result`, `List`, `Map`, `Set`, smart pointers.
- `10_ownership.arden`: ownership and borrowing examples.
- `11_lambdas.arden`: lambdas, closures, higher-order functions.
- `12_string_interp.arden`: string interpolation patterns.
- `13_error_handling.arden`: `Option`, `Result`, `?`, `require`.
- `14_async.arden`: async/await and async blocks.
- `15_stdlib.arden`: stdlib overview usage.
- `16_pattern_matching.arden`: pattern matching flows.
- `17_comprehensive.arden`: mixed end-to-end language usage.
- `18_file_io.arden`: file operations.
- `19_time.arden`: time functions.
- `20_system.arden`: system calls/utilities.
- `21_conversions.arden`: type conversions.
- `22_args.arden`: command-line args.
- `23_str_utils.arden`: string utilities.
- `24_test_attributes.arden`: test attributes and assertions.
- `25_range_types.arden`: `Range<T>` and iterators.
- `26_effect_system.arden`: effect attributes (`@Pure`, `@Io`, `@Thread`).
- `27_extern_c_interop.arden`: `extern function` C interop.
- `28_async_runtime_control.arden`: true async runtime task controls (`is_done`, `await_timeout`, `cancel`) with portable timeout behavior across Linux, macOS, and Windows.
- `29_effect_inference_and_any.arden`: automatic effect inference and `@Any`.
- `30_extern_variadic_printf.arden`: variadic `extern` calls (`printf` with `...`).
- `31_extern_abi_link_name.arden`: explicit ABI + symbol aliasing (`extern(c, "symbol")`).
- `32_extern_safe_wrapper.arden`: safe Arden wrapper around raw extern call.
- `33_extern_ptr_types.arden`: `Ptr<T>` extern signatures (`malloc`/`free` style APIs).
- `34_bindgen_workflow.arden`: usage style for generated `arden bindgen` extern declarations.
- `35_visibility_enforcement.arden`: enforced `public`/`private`/`protected` access rules.
- `36_inheritance_extends.arden`: class inheritance with `extends` and inherited methods.
- `37_interfaces_contracts.arden`: enforced `implements` contracts and interface-typed params.
- `38_import_aliases.arden`: `import ... as ...` alias usage (`math`, `str`, `io`).
- `39_compound_assign.arden`: compound assignment operators (`+=`, `-=`, `*=`, `/=`) for vars and fields.
- `40_borrow_scope_recovery.arden`: borrow checker scope edge case where immutable borrow ends and move is valid again.

## Full apps

- `app_bank.arden`
- `app_calculator.arden`
- `app_data_structures.arden`
- `app_game.arden`
- `app_notes.arden`
- `app_todo.arden`

## Multi-file projects

- `multi_file_project/`
- `multi_file_depth_project/`
- `insane_showcase_project/`
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
