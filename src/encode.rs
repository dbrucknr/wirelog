pub(crate) fn append_key(buf: &mut Vec<u8>, key: &str) {
    buf.push(b',');
    buf.push(b'"');
    write_escaped(buf, key);
    buf.extend_from_slice(b"\":");
}

pub(crate) fn append_str_val(buf: &mut Vec<u8>, val: &str) {
    buf.push(b'"');
    write_escaped(buf, val);
    buf.push(b'"');
}

pub(crate) fn write_escaped(buf: &mut Vec<u8>, s: &str) {
    for byte in s.bytes() {
        match byte {
            b'"' => buf.extend_from_slice(b"\\\""),
            b'\\' => buf.extend_from_slice(b"\\\\"),
            b'\n' => buf.extend_from_slice(b"\\n"),
            b'\r' => buf.extend_from_slice(b"\\r"),
            b'\t' => buf.extend_from_slice(b"\\t"),
            0x08 => buf.extend_from_slice(b"\\b"),
            0x0c => buf.extend_from_slice(b"\\f"),
            b if b < 0x20 => {
                buf.extend_from_slice(b"\\u00");
                buf.push(b"0123456789abcdef"[(b >> 4) as usize]);
                buf.push(b"0123456789abcdef"[(b & 0xf) as usize]);
            }
            _ => buf.push(byte),
        }
    }
}
