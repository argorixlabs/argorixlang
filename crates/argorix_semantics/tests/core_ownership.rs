//! Ownership of resource values in Argorix Core (ESP-009).
//!
//! A `Buffer`, or a struct, enum or array holding one, owns storage. It moves
//! when it is bound, passed, returned, stored or matched, and the checker
//! refuses every use that would read or release it twice.

use argorix_parser::core::parse_core_source;
use argorix_semantics::{check_core_program, CoreCheckOptions};

const PRELUDE: &str = "core 0.1;
module ownership.case;

struct Holder { values: Buffer<u8>, count: u64, }

fn take(b: Buffer<u8>) -> u64 { b.length() }

fn make() -> Buffer<u8> { let b: Buffer<u8> = Buffer::new(); b }
";

/// The diagnostic codes of `main`'s body, or an empty list if it checks.
fn codes(body: &str) -> Vec<String> {
    let source = format!("{PRELUDE}\npub fn argorix_main() -> u64 {{\n{body}\n}}\n");
    let program = parse_core_source(&source).expect("case parses");
    match check_core_program(&program, &CoreCheckOptions::default()) {
        Ok(()) => Vec::new(),
        Err(diagnostics) => diagnostics.into_iter().map(|item| item.code).collect(),
    }
}

#[test]
fn a_moved_buffer_cannot_be_used() {
    assert_eq!(
        codes(
            "let b: Buffer<u8> = Buffer::new();
             let a: Buffer<u8> = b;
             b.length() + a.length()"
        ),
        ["UseAfterMove"]
    );
}

#[test]
fn a_buffer_moved_on_one_branch_cannot_be_used_after_it() {
    assert_eq!(
        codes(
            "let b: Buffer<u8> = Buffer::new();
             let c: bool = 1u64 < 2u64;
             let mut t: u64 = 0u64;
             if c { t = take(b); }
             t + b.length()"
        ),
        ["UseAfterMove"]
    );
}

#[test]
fn a_buffer_moved_only_on_the_right_of_and_is_maybe_moved() {
    assert_eq!(
        codes(
            "let b: Buffer<u8> = Buffer::new();
             let c: bool = (1u64 < 2u64) && (take(b) > 0u64);
             if c { 1u64 } else { b.length() }"
        ),
        ["UseAfterMove"]
    );
}

#[test]
fn a_buffer_from_before_a_loop_cannot_be_moved_inside_it() {
    assert_eq!(
        codes(
            "let b: Buffer<u8> = Buffer::new();
             let mut i: u64 = 0u64;
             let mut t: u64 = 0u64;
             while i < 3u64 { t += take(b); i += 1u64; }
             t"
        ),
        ["MoveInLoop"]
    );
}

#[test]
fn a_move_that_leaves_the_loop_is_fine() {
    assert!(codes(
        "let b: Buffer<u8> = Buffer::new();
         let mut i: u64 = 0u64;
         while i < 3u64 {
             if i == 1u64 { return take(b); }
             i += 1u64;
         }
         0u64"
    )
    .is_empty());
    assert!(codes(
        "let b: Buffer<u8> = Buffer::new();
         let t: u64 = loop { break take(b); };
         t"
    )
    .is_empty());
}

#[test]
fn a_buffer_moved_on_both_branches_is_fine_if_not_used_after() {
    assert!(codes(
        "let b: Buffer<u8> = Buffer::new();
         let c: bool = 1u64 < 2u64;
         if c { take(b) } else { take(b) }"
    )
    .is_empty());
}

#[test]
fn a_moved_local_owns_again_once_assigned() {
    assert!(codes(
        "let mut b: Buffer<u8> = Buffer::new();
         let t: u64 = take(b);
         b = make();
         b.push(1u8);
         b.length() + t"
    )
    .is_empty());
}

#[test]
fn a_buffer_cannot_be_moved_out_of_a_field_or_an_element() {
    assert_eq!(
        codes(
            "let h: Holder = Holder { values: Buffer::new(), count: 1u64 };
             let v: Buffer<u8> = h.values;
             v.length()"
        ),
        ["MoveOutOfPlace"]
    );
    assert_eq!(
        codes(
            "let mut rows: Buffer<Buffer<u8>> = Buffer::new();
             rows.push(make());
             take(rows[0u64])"
        ),
        ["MoveOutOfPlace"]
    );
}

#[test]
fn a_field_or_an_element_can_be_inspected_in_place() {
    assert!(codes(
        "let mut h: Holder = Holder { values: Buffer::new(), count: 1u64 };
         h.values.push(4u8);
         let mut rows: Buffer<Buffer<u8>> = Buffer::new();
         rows.push(make());
         h.values.length() + rows[0u64].length() + h.count"
    )
    .is_empty());
}

#[test]
fn a_resource_temporary_cannot_be_inspected() {
    assert_eq!(codes("make().length()"), ["ResourceTemporary"]);
}

#[test]
fn a_resource_cannot_live_in_an_arena() {
    assert_eq!(
        codes(
            "let mut a: Arena<Holder> = Arena::new();
             let h: Handle<Holder> = a.alloc(Holder { values: Buffer::new(), count: 1u64 });
             h.count"
        ),
        ["ResourceInArena"]
    );
}

#[test]
fn plain_data_is_copied_not_moved() {
    assert!(codes(
        "let x: u64 = 3u64;
         let y: u64 = x;
         let z: u64 = x;
         x + y + z"
    )
    .is_empty());
}
