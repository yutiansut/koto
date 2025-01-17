//! The `string` core library module

pub mod format;
pub mod iterators;

use {
    super::iterator::collect_pair, crate::prelude::*, std::convert::TryFrom,
    unicode_segmentation::UnicodeSegmentation,
};

/// Initializes the `string` core library module
pub fn make_module() -> ValueMap {
    use Value::*;

    let result = ValueMap::new();

    result.add_fn("bytes", |vm, args| match vm.get_args(args) {
        [Str(s)] => {
            let result = iterators::Bytes::new(s.clone());
            Ok(ValueIterator::new(result).into())
        }
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("chars", |vm, args| match vm.get_args(args) {
        [Str(s)] => Ok(Iterator(ValueIterator::with_string(s.clone()))),
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("contains", |vm, args| match vm.get_args(args) {
        [Str(s1), Str(s2)] => Ok(s1.contains(s2.as_str()).into()),
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("ends_with", |vm, args| match vm.get_args(args) {
        [Str(s), Str(pattern)] => Ok(s.as_str().ends_with(pattern.as_str()).into()),
        unexpected => expected_two_strings_error(unexpected),
    });

    result.add_fn("escape", |vm, args| match vm.get_args(args) {
        [Str(s)] => Ok(s.escape_default().to_string().into()),
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("format", |vm, args| match vm.get_args(args) {
        [result @ Str(_)] => Ok(result.clone()),
        [Str(format), format_args @ ..] => {
            let format = format.clone();
            let format_args = format_args.to_vec();
            match format::format_string(vm, &format, &format_args) {
                Ok(result) => Ok(result.into()),
                Err(error) => Err(error),
            }
        }
        unexpected => type_error_with_slice(
            "a String as argument, followed by optional additional Values",
            unexpected,
        ),
    });

    result.add_fn("from_bytes", |vm, args| match vm.get_args(args) {
        [iterable] if iterable.is_iterable() => {
            let iterable = iterable.clone();
            let iterator = vm.make_iterator(iterable)?;
            let (size_hint, _) = iterator.size_hint();
            let mut bytes = Vec::<u8>::with_capacity(size_hint);

            for output in iterator.map(collect_pair) {
                use ValueIteratorOutput as Output;
                match output {
                    Output::Value(Number(n)) => match u8::try_from(n.as_i64()) {
                        Ok(byte) => bytes.push(byte),
                        Err(_) => return runtime_error!("'{n}' is out of the valid byte range"),
                    },
                    Output::Value(unexpected) => return type_error("a number", &unexpected),
                    Output::Error(error) => return Err(error),
                    _ => unreachable!(),
                }
            }

            match String::from_utf8(bytes) {
                Ok(result) => Ok(result.into()),
                Err(_) => runtime_error!("Input failed UTF-8 validation"),
            }
        }
        unexpected => type_error_with_slice("an iterable value as argument", unexpected),
    });

    result.add_fn("is_empty", |vm, args| match vm.get_args(args) {
        [Str(s)] => Ok(s.is_empty().into()),
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("lines", |vm, args| match vm.get_args(args) {
        [Str(s)] => {
            let result = iterators::Lines::new(s.clone());
            Ok(ValueIterator::new(result).into())
        }
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("replace", |vm, args| match vm.get_args(args) {
        [Str(input), Str(pattern), Str(replace)] => {
            Ok(input.replace(pattern.as_str(), replace).into())
        }
        unexpected => type_error_with_slice("three Strings as arguments", unexpected),
    });

    result.add_fn("size", |vm, args| match vm.get_args(args) {
        [Str(s)] => Ok(s.graphemes(true).count().into()),
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("split", |vm, args| {
        let iterator = match vm.get_args(args) {
            [Str(input), Str(pattern)] => {
                let result = iterators::Split::new(input.clone(), pattern.clone());
                ValueIterator::new(result)
            }
            [Str(input), predicate] if predicate.is_callable() => {
                let result = iterators::SplitWith::new(
                    input.clone(),
                    predicate.clone(),
                    vm.spawn_shared_vm(),
                );
                ValueIterator::new(result)
            }
            unexpected => {
                return type_error_with_slice(
                    "a String and either a String or predicate Function as arguments",
                    unexpected,
                )
            }
        };

        Ok(Iterator(iterator))
    });

    result.add_fn("starts_with", |vm, args| match vm.get_args(args) {
        [Str(s), Str(pattern)] => Ok(s.as_str().starts_with(pattern.as_str()).into()),
        unexpected => expected_two_strings_error(unexpected),
    });

    result.add_fn("to_lowercase", |vm, args| match vm.get_args(args) {
        [Str(s)] => {
            let result = s.chars().flat_map(|c| c.to_lowercase()).collect::<String>();
            Ok(result.into())
        }
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("to_number", |vm, args| match vm.get_args(args) {
        [Str(s)] => match s.parse::<i64>() {
            Ok(n) => Ok(Number(n.into())),
            Err(_) => match s.parse::<f64>() {
                Ok(n) => Ok(Number(n.into())),
                Err(_) => {
                    runtime_error!("string.to_number: Failed to convert '{s}'")
                }
            },
        },
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("to_uppercase", |vm, args| match vm.get_args(args) {
        [Str(s)] => {
            let result = s.chars().flat_map(|c| c.to_uppercase()).collect::<String>();
            Ok(result.into())
        }
        unexpected => expected_string_error(unexpected),
    });

    result.add_fn("trim", |vm, args| match vm.get_args(args) {
        [Str(s)] => {
            let result = match s.find(|c: char| !c.is_whitespace()) {
                Some(start) => {
                    let end = s.rfind(|c: char| !c.is_whitespace()).unwrap();
                    s.with_bounds(start..(end + 1)).unwrap()
                }
                None => s.with_bounds(0..0).unwrap(),
            };

            Ok(result.into())
        }
        unexpected => expected_string_error(unexpected),
    });

    result
}

fn expected_string_error(unexpected: &[Value]) -> RuntimeResult {
    type_error_with_slice("a String as argument", unexpected)
}

fn expected_two_strings_error(unexpected: &[Value]) -> RuntimeResult {
    type_error_with_slice("two Strings as arguments", unexpected)
}
