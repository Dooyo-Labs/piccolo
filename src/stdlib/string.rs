use crate::{Callback, CallbackReturn, Context, FromValue, IntoValue as _, String, Table, Value};

mod format;

pub fn load_string<'gc>(ctx: Context<'gc>) {
    let string = Table::new(&ctx);

    string.set_field(
        ctx,
        "len",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let string = stack.consume::<String>(ctx)?;
            let len = string.len();
            stack.replace(ctx, len);
            Ok(CallbackReturn::Return)
        }),
    );

    string.set_field(
        ctx,
        "byte",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (string, i, j) = stack.consume::<(String, Option<i64>, Option<i64>)>(ctx)?;
            let i = i.unwrap_or(1);
            let substr = sub(string.as_bytes(), i, j.or(Some(i)))?;
            stack.extend(substr.iter().map(|b| Value::Integer(i64::from(*b))));
            Ok(CallbackReturn::Return)
        }),
    );

    string.set_field(
        ctx,
        "char",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let string = ctx.intern(
                &stack
                    .into_iter()
                    .map(|c| u8::from_value(ctx, c))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            stack.replace(ctx, string);
            Ok(CallbackReturn::Return)
        }),
    );

    string.set_field(
        ctx,
        "find",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (s, pattern, init, plain) =
                stack.consume::<(String, String, Option<i64>, Option<bool>)>(ctx)?;

            let plain = plain.unwrap_or_default();
            if !plain && is_special_pattern(pattern.as_bytes()) {
                return Err("TODO: Implement find() pattern matching"
                    .into_value(ctx)
                    .into());
            }

            let s_bytes = s.as_bytes();
            let pattern_bytes = pattern.as_bytes();

            let start_pos = rel_pos_to_index(init.unwrap_or(1), s_bytes.len());

            if start_pos >= s_bytes.len() || (s_bytes.len() - start_pos) < pattern_bytes.len() {
                stack.replace(ctx, Value::Nil);
                return Ok(CallbackReturn::Return);
            }

            if pattern_bytes.is_empty() {
                stack.replace(ctx, (start_pos as i64 + 1, start_pos as i64));
                return Ok(CallbackReturn::Return);
            }

            if let Some(pos) = s_bytes[start_pos..]
                .windows(pattern_bytes.len())
                .position(|window| window == pattern_bytes)
            {
                let start = start_pos + pos + 1;
                let end = start_pos + pos + pattern_bytes.len();
                stack.replace(ctx, (start as i64, end as i64));
            } else {
                stack.replace(ctx, Value::Nil);
            }

            Ok(CallbackReturn::Return)
        }),
    );

    string.set_field(
        ctx,
        "sub",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let (string, i, j) = stack.consume::<(String, i64, Option<i64>)>(ctx)?;
            let substr = ctx.intern(sub(string.as_bytes(), i, j)?);
            stack.replace(ctx, substr);
            Ok(CallbackReturn::Return)
        }),
    );

    string.set_field(
        ctx,
        "lower",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let string = stack.consume::<String>(ctx)?;
            let lowered = ctx.intern(
                &string
                    .as_bytes()
                    .iter()
                    .map(u8::to_ascii_lowercase)
                    .collect::<Vec<_>>(),
            );
            stack.replace(ctx, lowered);
            Ok(CallbackReturn::Return)
        }),
    );

    string.set_field(
        ctx,
        "reverse",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let string = stack.consume::<String>(ctx)?;
            let reversed = ctx.intern(&string.as_bytes().iter().copied().rev().collect::<Vec<_>>());
            stack.replace(ctx, reversed);
            Ok(CallbackReturn::Return)
        }),
    );

    string.set_field(
        ctx,
        "upper",
        Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let string = stack.consume::<String>(ctx)?;
            let uppered = ctx.intern(
                &string
                    .as_bytes()
                    .iter()
                    .map(u8::to_ascii_uppercase)
                    .collect::<Vec<_>>(),
            );
            stack.replace(ctx, uppered);
            Ok(CallbackReturn::Return)
        }),
    );

    string
        .set(
            ctx,
            "format",
            Callback::from_fn(&ctx, |ctx, _, stack| {
                let seq = format::string_format(ctx, stack)?;
                Ok(CallbackReturn::Sequence(crate::BoxSequence::new(&ctx, seq)))
            }),
        )
        .unwrap();

    ctx.set_global("string", string);
}

/// Converts base-1 string position, (where negative values count from the end of the string),
/// to a base-0 index.
///
/// Like PUC-Lua, an out-of-bounds position of 0 is an alias for 1
/// Like PUC-Lua, it clamps negative offsets to the start of the string
///
/// Note: This does _not_ clamp the index to the string length.
fn rel_pos_to_index(pos: i64, len: usize) -> usize {
    match pos {
        pos if pos > 0 => pos.saturating_sub(1).try_into().unwrap_or(usize::MAX),
        0 => 0,
        pos => len.saturating_sub(pos.unsigned_abs().try_into().unwrap_or(usize::MAX)),
    }
}

fn is_special_pattern(pattern: &[u8]) -> bool {
    const SPECIAL_BYTES: &[u8] = b"^$*+?.([%-";
    pattern.iter().any(|&b| SPECIAL_BYTES.contains(&b))
}

fn sub(string: &[u8], i: i64, j: Option<i64>) -> Result<&[u8], std::num::TryFromIntError> {
    let i = match i {
        i if i > 0 => i.saturating_sub(1).try_into()?,
        0 => 0,
        i => string.len().saturating_sub(i.unsigned_abs().try_into()?),
    };
    let j = if let Some(j) = j {
        if j >= 0 {
            j.try_into()?
        } else {
            let j: usize = j.unsigned_abs().try_into()?;
            string.len().saturating_sub(j.saturating_sub(1))
        }
    } else {
        string.len()
    }
    .clamp(0, string.len());

    Ok(if i >= j || i >= string.len() {
        &[]
    } else {
        &string[i..j]
    })
}
