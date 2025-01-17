mod runtime {
    use {
        koto_bytecode::{Chunk, Loader},
        koto_runtime::Vm,
    };

    fn check_script_fails(script: &str) {
        let mut vm = Vm::default();

        let print_chunk = |script: &str, chunk| {
            println!("{}\n", script);
            let script_lines = script.lines().collect::<Vec<_>>();

            println!("{}", Chunk::instructions_as_string(chunk, &script_lines));
        };

        let mut loader = Loader::default();
        let chunk = match loader.compile_script(script, &None) {
            Ok(chunk) => chunk,
            Err(error) => {
                print_chunk(script, vm.chunk());
                panic!("Error while compiling script: {}", error);
            }
        };

        if let Ok(result) = vm.run(chunk) {
            print_chunk(script, vm.chunk());
            panic!("Script didn't fail as expected, result: {}", result)
        }
    }

    mod should_fail {
        use super::*;

        mod assertions {
            use super::*;

            #[test]
            fn check_assert() {
                check_script_fails("assert false");
            }

            #[test]
            fn check_assert_eq() {
                check_script_fails("assert_eq 0, 1");
            }

            #[test]
            fn check_assert_ne() {
                check_script_fails("assert_ne 1, 1");
            }

            #[test]
            fn check_assert_near() {
                check_script_fails("assert_near 1, 2, 0.1");
            }
        }

        mod missing_values {
            use super::*;

            #[test]
            fn missing_identifier_before_last_expression() {
                let script = "
x = 123
y
x
";
                check_script_fails(script);
            }
        }

        mod iterators {
            use super::*;

            #[test]
            fn iterator_consume_should_propagate_error() {
                let script = "
(1..5)
  .each |_| assert false
  .consume()
";
                check_script_fails(script);
            }

            #[test]
            fn iterator_count_should_propagate_error() {
                let script = "
(1..5)
  .each |_| assert false
  .count()
";
                check_script_fails(script);
            }
        }

        mod function_calls {
            use super::*;

            #[test]
            fn tuple_unpacking_of_non_tuple() {
                let script = r#"
f = |(a, b)| a + b
f "O_o"
"#;
                check_script_fails(script);
            }

            #[test]
            fn tuple_unpacking_of_tuple_with_wrong_size() {
                let script = r#"
f = |(a, b)| a + b
f (1, 2, 3)
"#;
                check_script_fails(script);
            }

            #[test]
            fn list_unpacking_of_non_list() {
                let script = r#"
f = |[a, b]| a + b
f (1, 2)
"#;
                check_script_fails(script);
            }

            #[test]
            fn list_unpacking_of_list_with_wrong_size() {
                let script = r#"
f = |[a, b]| a + b
f [1, 2, 3]
"#;
                check_script_fails(script);
            }

            #[test]
            fn capturing_a_reserved_value_in_a_temporary_function() {
                let script = "
x = (1..10).find |n| n == x
";
                check_script_fails(script);
            }
        }

        mod indexing {
            use super::*;

            #[test]
            fn num2_mutation_via_index() {
                let script = "
x = make_num2 1, 2
x[0] = -1
";
                check_script_fails(script);
            }

            #[test]
            fn num4_mutation_via_index() {
                let script = "
x = make_num4 1, 2, 3, 4
x[0] = -1
";
                check_script_fails(script);
            }
        }
    }
}
