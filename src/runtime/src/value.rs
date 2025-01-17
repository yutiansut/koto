//! The core value type used in the Koto runtime

use {
    crate::{
        num2, num4, value_key::ValueRef, value_map::ValueMap, ExternalFunction, ExternalValue,
        MetaKey, ValueIterator, ValueList, ValueNumber, ValueString, ValueTuple, ValueVec,
    },
    koto_bytecode::Chunk,
    std::{fmt, rc::Rc},
};

/// The core Value type for Koto
#[derive(Clone, Debug)]
pub enum Value {
    /// The default type representing the absence of a value
    Null,

    /// A boolean, can be either true or false
    Bool(bool),

    /// A number, represented as either a signed 64 bit integer or float
    Number(ValueNumber),

    /// A pair of 64 bit floats, useful when working with 2 dimensional values
    Num2(num2::Num2),

    /// A pack of four 32 bit floats, useful in working with 3 or 4 dimensional values
    Num4(num4::Num4),

    /// A range with start/end boundaries
    Range(IntRange),

    /// The list type used in Koto
    List(ValueList),

    /// The tuple type used in Koto
    Tuple(ValueTuple),

    /// The hash map type used in Koto
    Map(ValueMap),

    /// The string type used in Koto
    Str(ValueString),

    /// A callable function with simple properties
    SimpleFunction(SimpleFunctionInfo),

    /// A callable function with less simple properties, e.g. captures, instance function, etc.
    Function(FunctionInfo),

    /// A function that produces an Iterator when called
    ///
    /// A [Vm](crate::Vm) gets spawned for the function to run in, which pauses each time a yield
    /// instruction is encountered. See Vm::call_generator and Iterable::Generator.
    Generator(FunctionInfo),

    /// The iterator type used in Koto
    Iterator(ValueIterator),

    /// A function that's defined outside of the Koto runtime
    ExternalFunction(ExternalFunction),

    /// A value type that's defined outside of the Koto runtime
    ExternalValue(ExternalValue),

    /// The range type used as a temporary value in index expressions.
    ///
    /// Note: this is intended for internal use only.
    IndexRange(IndexRange),

    /// A tuple of values that are packed into a contiguous series of registers
    ///
    /// Used as an optimization when multiple values are passed around without being assigned to a
    /// single Tuple value.
    ///
    /// Note: this is intended for internal use only.
    TemporaryTuple(RegisterSlice),

    /// The builder used while building lists or tuples
    ///
    /// Note: this is intended for internal use only.
    SequenceBuilder(Vec<Value>),

    /// The string builder used during string interpolation
    ///
    /// Note: this is intended for internal use only.
    StringBuilder(String),
}

impl Value {
    #[inline]
    pub(crate) fn as_ref(&self) -> ValueRef {
        match &self {
            Value::Null => ValueRef::Null,
            Value::Bool(b) => ValueRef::Bool(b),
            Value::Number(n) => ValueRef::Number(n),
            Value::Num2(n) => ValueRef::Num2(n),
            Value::Num4(n) => ValueRef::Num4(n),
            Value::Str(s) => ValueRef::Str(s),
            Value::Range(r) => ValueRef::Range(r),
            _ => unreachable!(), // Only immutable values can be used in ValueKey
        }
    }

    /// Returns a recursive 'deep copy' of a Value
    ///
    /// This is used by the various `.deep_copy()` core library functions.
    #[must_use]
    pub fn deep_copy(&self) -> Value {
        use Value::*;

        match &self {
            List(l) => {
                let result = l.data().iter().map(|v| v.deep_copy()).collect::<ValueVec>();
                List(ValueList::with_data(result))
            }
            Tuple(t) => {
                let result = t.iter().map(|v| v.deep_copy()).collect::<Vec<_>>();
                Tuple(result.into())
            }
            Map(m) => {
                let data = m
                    .data()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.deep_copy()))
                    .collect();
                let meta = m.meta_map().map(|meta| meta.borrow().clone());
                Map(ValueMap::with_contents(data, meta))
            }
            Iterator(i) => Iterator(i.make_copy()),
            _ => self.clone(),
        }
    }

    /// Returns true if the value has function-like callable behaviour
    pub fn is_callable(&self) -> bool {
        use Value::*;
        matches!(self, SimpleFunction(_) | Function(_) | ExternalFunction(_))
    }

    /// Returns true if the value doesn't have internal mutability
    ///
    /// Only immutable values are acceptable as map keys.
    pub fn is_immutable(&self) -> bool {
        use Value::*;
        matches!(
            self,
            Null | Bool(_) | Number(_) | Num2(_) | Num4(_) | Range(_) | Str(_)
        )
    }

    /// Returns true if a `ValueIterator` can be made from the value
    pub fn is_iterable(&self) -> bool {
        use Value::*;
        matches!(
            self,
            Num2(_) | Num4(_) | Range(_) | List(_) | Tuple(_) | Map(_) | Str(_) | Iterator(_)
        )
    }

    /// Returns the 'size' of the value
    ///
    /// A value's size is the number of elements that can used in unpacking expressions
    /// e.g.
    /// x = [1, 2, 3] # x has size 3
    /// a, b, c = x
    ///
    /// See:
    ///   - [Op::Size](koto_bytecode::Op::Size)
    ///   - [Op::CheckSizeEqual](koto_bytecode::Op::CheckSizeEqual).
    ///   - [Op::CheckSizeMin](koto_bytecode::Op::CheckSizeMin).
    pub fn size(&self) -> usize {
        use Value::*;

        match &self {
            List(l) => l.len(),
            Tuple(t) => t.len(),
            TemporaryTuple(RegisterSlice { count, .. }) => *count as usize,
            Map(m) => m.len(),
            Num2(_) => 2,
            Num4(_) => 4,
            _ => 1,
        }
    }

    /// Returns the value's type as a ValueString
    pub fn type_as_string(&self) -> ValueString {
        use Value::*;
        match &self {
            Null => TYPE_NULL.with(|x| x.clone()),
            Bool(_) => TYPE_BOOL.with(|x| x.clone()),
            Number(ValueNumber::F64(_)) => TYPE_FLOAT.with(|x| x.clone()),
            Number(ValueNumber::I64(_)) => TYPE_INT.with(|x| x.clone()),
            Num2(_) => TYPE_NUM2.with(|x| x.clone()),
            Num4(_) => TYPE_NUM4.with(|x| x.clone()),
            List(_) => TYPE_LIST.with(|x| x.clone()),
            Range { .. } => TYPE_RANGE.with(|x| x.clone()),
            IndexRange { .. } => TYPE_INDEX_RANGE.with(|x| x.clone()),
            Map(m) => match m.get_meta_value(&MetaKey::Type) {
                Some(Str(s)) => s,
                Some(_) => "Error: expected string for overloaded type".into(),
                None => TYPE_MAP.with(|x| x.clone()),
            },
            Str(_) => TYPE_STRING.with(|x| x.clone()),
            Tuple(_) => TYPE_TUPLE.with(|x| x.clone()),
            SimpleFunction(_) | Function(_) => TYPE_FUNCTION.with(|x| x.clone()),
            Generator(_) => TYPE_GENERATOR.with(|x| x.clone()),
            ExternalFunction(_) => TYPE_EXTERNAL_FUNCTION.with(|x| x.clone()),
            ExternalValue(value) => value.value_type(),
            Iterator(_) => TYPE_ITERATOR.with(|x| x.clone()),
            TemporaryTuple { .. } => TYPE_TEMPORARY_TUPLE.with(|x| x.clone()),
            SequenceBuilder(_) => TYPE_SEQUENCE_BUILDER.with(|x| x.clone()),
            StringBuilder(_) => TYPE_STRING_BUILDER.with(|x| x.clone()),
        }
    }
}

thread_local! {
    static TYPE_NULL: ValueString = "Null".into();
    static TYPE_BOOL: ValueString = "Bool".into();
    static TYPE_FLOAT: ValueString = "Float".into();
    static TYPE_INT: ValueString = "Int".into();
    static TYPE_NUM2: ValueString = "Num2".into();
    static TYPE_NUM4: ValueString = "Num4".into();
    static TYPE_LIST: ValueString = "List".into();
    static TYPE_RANGE: ValueString = "Range".into();
    static TYPE_INDEX_RANGE: ValueString = "IndexRange".into();
    static TYPE_MAP: ValueString = "Map".into();
    static TYPE_STRING: ValueString = "String".into();
    static TYPE_TUPLE: ValueString = "Tuple".into();
    static TYPE_FUNCTION: ValueString = "Function".into();
    static TYPE_GENERATOR: ValueString = "Generator".into();
    static TYPE_EXTERNAL_FUNCTION: ValueString = "ExternalFunction".into();
    static TYPE_ITERATOR: ValueString = "Iterator".into();
    static TYPE_TEMPORARY_TUPLE: ValueString = "TemporaryTuple".into();
    static TYPE_SEQUENCE_BUILDER: ValueString = "SequenceBuilder".into();
    static TYPE_STRING_BUILDER: ValueString = "StringBuilder".into();
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Value::*;
        match self {
            Null => f.write_str("null"),
            Bool(b) => write!(f, "{b}"),
            Number(n) => write!(f, "{n}"),
            Num2(n) => write!(f, "{n}"),
            Num4(n) => write!(f, "{n}"),
            Str(s) => {
                if f.alternate() {
                    write!(f, "{s:#}")
                } else {
                    write!(f, "{s}")
                }
            }
            List(l) => write!(f, "{l}"),
            Tuple(t) => write!(f, "{t}"),
            Map(m) => {
                if f.alternate() {
                    write!(f, "{m:#}")
                } else {
                    write!(f, "{m}")
                }
            }
            Range(IntRange { start, end }) => write!(f, "{start}..{end}"),
            SimpleFunction(_) | Function(_) => write!(f, "||"),
            Generator(_) => write!(f, "Generator"),
            Iterator(_) => write!(f, "Iterator"),
            ExternalFunction(_) => write!(f, "||"),
            ExternalValue(_) => f.write_str(&self.type_as_string()),
            IndexRange(self::IndexRange { .. }) => f.write_str("IndexRange"),
            TemporaryTuple(RegisterSlice { start, count }) => {
                write!(f, "TemporaryTuple [{start}..{}]", start + count)
            }
            SequenceBuilder(_) => write!(f, "SequenceBuilder"),
            StringBuilder(s) => write!(f, "StringBuilder({s})"),
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<ValueNumber> for Value {
    fn from(value: ValueNumber) -> Self {
        Self::Number(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Str(value.into())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Str(value.into())
    }
}

impl From<ValueString> for Value {
    fn from(value: ValueString) -> Self {
        Self::Str(value)
    }
}

impl From<ExternalValue> for Value {
    fn from(value: ExternalValue) -> Self {
        Self::ExternalValue(value)
    }
}

impl From<ValueIterator> for Value {
    fn from(value: ValueIterator) -> Self {
        Self::Iterator(value)
    }
}

/// A plain and simple function
///
/// See also:
/// * [Value::SimpleFunction]
/// * [FunctionInfo]
#[derive(Clone, Debug)]
pub struct SimpleFunctionInfo {
    /// The [Chunk] in which the function can be found.
    pub chunk: Rc<Chunk>,
    /// The start ip of the function.
    pub ip: usize,
    /// The expected number of arguments for the function
    pub arg_count: u8,
}

/// A fully-featured function with all the bells and whistles
///
/// See also:
/// * [Value::Function]
/// * [SimpleFunctionInfo]
#[derive(Clone, Debug)]
pub struct FunctionInfo {
    /// The [Chunk] in which the function can be found.
    pub chunk: Rc<Chunk>,
    /// The start ip of the function.
    pub ip: usize,
    /// The expected number of arguments for the function.
    pub arg_count: u8,
    /// If the function is an instance function, then the first argument will be `self`.
    pub instance_function: bool,
    /// If the function is variadic, then extra args will be captured in a tuple.
    pub variadic: bool,
    /// If the function has a single arg, and that arg is an unpacked tuple
    ///
    /// This is used to optimize external calls where the caller has a series of args that might be
    /// unpacked by the function, and it would be wasteful to create a Tuple when it's going to be
    /// immediately unpacked and discarded.
    pub arg_is_unpacked_tuple: bool,
    /// The optional list of captures that should be copied into scope when the function is called.
    //
    // Q. Why use a ValueList?
    // A. Because capturing values currently works by assigning by index, after the function
    //    itself has been created.
    // Q. Why not use a SequenceBuilder?
    // A. Recursive functions need to capture themselves into the list, and the captured function
    //    and the assigned function need to share the same captures list. Currently the only way
    //    for this to work is to allow mutation of the shared list after the creation of the
    //    function, so a ValueList is a reasonable choice.
    // Q. What about using Rc<[Value]> for non-recursive functions, or Option<Value> for
    //    non-recursive functions with a single capture?
    // A. These could be potential optimizations to investigate at some point, but would involve
    //    placing FunctionInfo behind an Rc due to its increased size, so it's not clear if there
    //    would be an overall performance win.
    pub captures: Option<ValueList>,
}

/// The integer range type that's exposed to users in the runtime
///
/// See [Value::Range]
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntRange {
    pub start: isize,
    pub end: isize,
}

impl IntRange {
    /// Returns true if the range's start is less than or equal to its end
    pub fn is_ascending(&self) -> bool {
        self.start <= self.end
    }

    /// Returns the size of the range
    ///
    /// Descending ranges have a non-negative size, i.e. the size is equal to `start - end`.
    pub fn len(&self) -> usize {
        if self.is_ascending() {
            (self.end - self.start) as usize
        } else {
            (self.start - self.end) as usize
        }
    }

    /// Returns true if the range's size is zero
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A range type that's used in indexing expressions
///
/// Index ranges have an optional end to support indexing expressions like `foo[10..]`.
///
/// See [Value::IndexRange]
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexRange {
    pub start: usize,
    pub end: Option<usize>,
}

/// A slice of a VM's registers
///
/// See [Value::TemporaryTuple]
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegisterSlice {
    pub start: u8,
    pub count: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_mem_size() {
        // All Value variants should have a size of <= 32 bytes, and with the variant flag the
        // total size of Value should not be greater than 40 bytes.
        assert!(std::mem::size_of::<Value>() <= 40);
    }
}
