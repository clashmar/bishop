use mlua::{Error, Integer, Value};

const MAX_EXACT_F64_INTEGER: u64 = 1 << 53;

/// Keeps Lua integer semantics when possible and otherwise uses only exact numbers.
pub(crate) fn lua_unsigned_id(id: u64) -> mlua::Result<Value> {
    if let Ok(value) = Integer::try_from(id) {
        return Ok(Value::Integer(value));
    }
    if id <= MAX_EXACT_F64_INTEGER {
        Ok(Value::Number(id as f64))
    } else {
        Err(Error::RuntimeError(format!(
            "ID {id} cannot be represented exactly in Luau"
        )))
    }
}

/// Converts persisted signed integers without truncation on 32-bit Luau targets.
pub(crate) fn lua_signed_integer(value: i64) -> mlua::Result<Value> {
    match Integer::try_from(value) {
        Ok(integer) => Ok(Value::Integer(integer)),
        Err(_) if value.unsigned_abs() <= MAX_EXACT_F64_INTEGER => {
            Ok(Value::Number(value as f64))
        }
        Err(_) => Err(Error::RuntimeError(format!(
            "saved integer {value} cannot be represented exactly in Luau"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{lua_signed_integer, lua_unsigned_id};
    use mlua::Value;

    #[test]
    fn small_ids_preserve_integer_semantics() -> mlua::Result<()> {
        assert!(matches!(lua_unsigned_id(42)?, Value::Integer(42)));
        assert!(matches!(lua_signed_integer(-42)?, Value::Integer(-42)));
        Ok(())
    }

    #[test]
    fn unsigned_ids_that_cannot_fit_integer_or_exact_number_fail() {
        assert!(lua_unsigned_id(u64::MAX).is_err());
    }

    #[test]
    fn ids_above_32_bit_lua_integer_range_remain_exact() -> mlua::Result<()> {
        let id = i32::MAX as u64 + 1;
        match lua_unsigned_id(id)? {
            Value::Integer(value) => assert_eq!(value as u64, id),
            Value::Number(value) => assert_eq!(value, id as f64),
            other => panic!("unexpected Lua ID representation: {other:?}"),
        }
        Ok(())
    }
}
