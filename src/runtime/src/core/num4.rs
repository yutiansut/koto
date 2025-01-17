//! The `num4` core library module

use {
    super::iterator::collect_pair,
    crate::{num4, prelude::*, ValueIteratorOutput as Output},
};

/// Initializes the `num4` core library module
pub fn make_module() -> ValueMap {
    use Value::*;

    let result = ValueMap::new();

    result.add_fn("length", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.length().into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("lerp", |vm, args| match vm.get_args(args) {
        [Num4(a), Num4(b), Number(t)] => {
            let result = *t * (b - a) + a;
            Ok(Num4(result))
        }
        unexpected => type_error_with_slice("(Num4, Num4, Number) as arguments", unexpected),
    });

    result.add_fn("make_num4", |vm, args| {
        let result = match vm.get_args(args) {
            [Number(n)] => num4::Num4(n.into(), n.into(), n.into(), n.into()),
            [Number(n1), Number(n2)] => num4::Num4(n1.into(), n2.into(), 0.0, 0.0),
            [Number(n1), Number(n2), Number(n3)] => {
                num4::Num4(n1.into(), n2.into(), n3.into(), 0.0)
            }
            [Number(n1), Number(n2), Number(n3), Number(n4)] => {
                num4::Num4(n1.into(), n2.into(), n3.into(), n4.into())
            }
            [Num2(n)] => num4::Num4(n[0] as f32, n[1] as f32, 0.0, 0.0),
            [Num4(n)] => *n,
            [iterable] if iterable.is_iterable() => {
                let iterable = iterable.clone();
                let iterator = vm.make_iterator(iterable)?;
                num4_from_iterator(iterator)?
            }
            unexpected => {
                return type_error_with_slice("Numbers or an iterable as arguments", unexpected)
            }
        };
        Ok(Num4(result))
    });

    result.add_fn("max", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number((n.0.max(n.1).max(n.2).max(n.3)).into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("min", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number((n.0.min(n.1).min(n.2).min(n.3)).into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("normalize", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Num4(n.normalize())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("product", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(
            (n.0 as f64 * n.1 as f64 * n.2 as f64 * n.3 as f64).into(),
        )),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("sum", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(
            (n.0 as f64 + n.1 as f64 + n.2 as f64 + n.3 as f64).into(),
        )),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("with", |vm, args| match vm.get_args(args) {
        [Num4(n), Number(i), Number(value)] => {
            let mut result = *n;
            match usize::from(i) {
                0 => result.0 = value.into(),
                1 => result.1 = value.into(),
                2 => result.2 = value.into(),
                3 => result.3 = value.into(),
                other => return runtime_error!("num4.with: invalid index '{other}'"),
            }
            Ok(Num4(result))
        }
        unexpected => num4_error(unexpected),
    });

    result.add_fn("r", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.0.into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("g", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.1.into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("b", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.2.into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("a", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.3.into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("x", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.0.into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("y", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.1.into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("z", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.2.into())),
        unexpected => num4_error(unexpected),
    });

    result.add_fn("w", |vm, args| match vm.get_args(args) {
        [Num4(n)] => Ok(Number(n.3.into())),
        unexpected => num4_error(unexpected),
    });

    result
}

fn num4_error(unexpected: &[Value]) -> RuntimeResult {
    type_error_with_slice("a Num4 as argument", unexpected)
}

pub(crate) fn num4_from_iterator(iterator: ValueIterator) -> Result<num4::Num4, RuntimeError> {
    let mut result = num4::Num4::default();
    for (i, value) in iterator.take(4).map(collect_pair).enumerate() {
        match value {
            Output::Value(Value::Number(n)) => result[i] = n.into(),
            Output::Value(unexpected) => return type_error_with_slice("a Number", &[unexpected]),
            Output::Error(e) => return Err(e),
            _ => unreachable!(), // ValuePairs collected in collect_pair
        }
    }
    Ok(result)
}
