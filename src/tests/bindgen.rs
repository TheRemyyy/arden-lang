use crate::bindgen::{generate_bindings, generate_from_prototype, strip_comments};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn parses_pointer_return_prototypes() {
    let generated = generate_from_prototype("char *strdup(const char *s)")
        .expect("pointer return prototype should parse");
    assert_eq!(generated, "extern(c) function strdup(s: String): String;");
}

#[test]
fn does_not_collapse_double_char_pointer_return_into_string() {
    let generated = generate_from_prototype("char **make_argv(void)")
        .expect("double char pointer return prototype should parse");
    assert_eq!(generated, "extern(c) function make_argv(): Ptr<None>;");
}

#[test]
fn skips_function_pointer_param_prototypes_entirely() {
    let generated = generate_from_prototype(
        "void qsort(void *base, size_t n, size_t sz, int (*cmp)(const void*, const void*))",
    );
    assert!(generated.is_none());
}

#[test]
fn keeps_tokens_separated_when_stripping_inline_block_comments() {
    let stripped = strip_comments("unsigned/*comment*/int count(void);");
    assert_eq!(stripped, "unsigned int count(void);");
}

#[test]
fn preserves_unsigned_type_normalization() {
    let generated = generate_from_prototype("unsigned short checksum(unsigned int value)")
        .expect("unsigned integer types should normalize correctly");
    assert_eq!(
        generated,
        "extern(c) function checksum(value: Integer): Integer;"
    );
}

#[test]
fn preserves_reordered_unsigned_type_normalization() {
    let generated = generate_from_prototype("long unsigned int checksum(long unsigned int value)")
        .expect("reordered unsigned integer types should normalize correctly");
    assert_eq!(
        generated,
        "extern(c) function checksum(value: Integer): Integer;"
    );
}

#[test]
fn preserves_plain_signed_type_normalization() {
    let generated = generate_from_prototype("signed negate(signed value)")
        .expect("plain signed integer types should normalize correctly");
    assert_eq!(
        generated,
        "extern(c) function negate(value: Integer): Integer;"
    );
}

#[test]
fn strips_restrict_qualifiers_from_pointer_params() {
    let generated = generate_from_prototype("void copy(char *restrict dst, char *restrict src)")
        .expect("restrict-qualified pointers should parse");
    assert_eq!(
        generated,
        "extern(c) function copy(dst: String, src: String): None;"
    );
}

#[test]
fn preserves_double_pointer_depth_with_restrict_qualifiers() {
    let generated = generate_from_prototype("void main_like(int argc, char **restrict argv)")
        .expect("restrict-qualified double pointers should parse");
    assert_eq!(
        generated,
        "extern(c) function main_like(argc: Integer, argv: Ptr<None>): None;"
    );
}

#[test]
fn bindgen_handles_comment_only_token_boundaries_in_real_header_flow() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let header_path = std::env::temp_dir().join(format!("arden_bindgen_{unique}.h"));
    let output_path = std::env::temp_dir().join(format!("arden_bindgen_{unique}.arden"));
    let header = "unsigned/*keep-space*/int count(void);\n";
    std::fs::write(&header_path, header).expect("temporary header should be written");

    let count =
        generate_bindings(&header_path, Some(&output_path)).expect("bindgen should succeed");
    let generated =
        std::fs::read_to_string(&output_path).expect("generated bindings should be readable");

    let _ = std::fs::remove_file(&header_path);
    let _ = std::fs::remove_file(&output_path);

    assert_eq!(count, 1);
    assert!(generated.contains("extern(c) function count(): Integer;"));
}

#[test]
fn array_parameters_decay_to_valid_arden_parameters() {
    let generated = generate_from_prototype("void fill(char name[16], int values[4])")
        .expect("array parameters should parse");
    assert_eq!(
        generated,
        "extern(c) function fill(name: String, values: Ptr<None>): None;"
    );
}

#[test]
fn does_not_collapse_double_char_pointer_params_into_string() {
    let generated = generate_from_prototype("void main_like(int argc, char **argv)")
        .expect("double char pointer params should parse");
    assert_eq!(
        generated,
        "extern(c) function main_like(argc: Integer, argv: Ptr<None>): None;"
    );
}

#[test]
fn bindgen_cli_emits_valid_identifiers_for_array_parameters() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let header_path = std::env::temp_dir().join(format!("arden_bindgen_arrays_{unique}.h"));
    let output_path = std::env::temp_dir().join(format!("arden_bindgen_arrays_{unique}.arden"));
    let header = "void fill(char name[16], int values[4]);\n";
    std::fs::write(&header_path, header).expect("temporary header should be written");

    let count =
        generate_bindings(&header_path, Some(&output_path)).expect("bindgen should succeed");
    let generated =
        std::fs::read_to_string(&output_path).expect("generated bindings should be readable");

    let _ = std::fs::remove_file(&header_path);
    let _ = std::fs::remove_file(&output_path);

    assert_eq!(count, 1);
    assert!(generated.contains("extern(c) function fill(name: String, values: Ptr<None>): None;"));
    assert!(!generated.contains("name[16]"));
    assert!(!generated.contains("values[4]"));
}

#[test]
fn inline_prototypes_are_not_dropped() {
    let generated =
        generate_from_prototype("static inline unsigned short checksum(unsigned int value)")
            .expect("inline prototype should parse");
    assert_eq!(
        generated,
        "extern(c) function checksum(value: Integer): Integer;"
    );
}

#[test]
fn spaced_array_parameters_preserve_real_parameter_names() {
    let generated = generate_from_prototype("void fill(char name [16], const char label [])")
        .expect("spaced array parameters should parse");
    assert_eq!(
        generated,
        "extern(c) function fill(name: String, label: String): None;"
    );
}
