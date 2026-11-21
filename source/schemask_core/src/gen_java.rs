use {
    crate::{
        Maskoid,
        MaskoidField,
        v1::{
            MaskoidConstString,
            MaskoidElement,
            MaskoidRef,
            SchemaskV1,
        },
    },
    heck::{
        ToLowerCamelCase,
        ToUpperCamelCase,
    },
    std::collections::{
        BTreeMap,
        BTreeSet,
    },
};

pub struct JavaFile {
    pub path: String,
    pub source: String,
}

pub struct JavaConfig {
    pub package: String,
}

const JSON_NODE: &str = "com.fasterxml.jackson.databind.JsonNode";
const NODE_FACTORY: &str = "com.fasterxml.jackson.databind.node.JsonNodeFactory.instance";
const OBJECT_NODE: &str = "com.fasterxml.jackson.databind.node.ObjectNode";
const ARRAY_NODE: &str = "com.fasterxml.jackson.databind.node.ArrayNode";
const OPTIONAL: &str = "java.util.Optional";

fn to_java_ident(cased: String) -> String {
    let reserved: &[&str] =
        &[
            "abstract",
            "assert",
            "boolean",
            "break",
            "byte",
            "case",
            "catch",
            "char",
            "class",
            "const",
            "continue",
            "default",
            "do",
            "double",
            "else",
            "enum",
            "extends",
            "final",
            "finally",
            "float",
            "for",
            "goto",
            "if",
            "implements",
            "import",
            "instanceof",
            "int",
            "interface",
            "long",
            "native",
            "new",
            "package",
            "private",
            "protected",
            "public",
            "return",
            "short",
            "static",
            "strictfp",
            "super",
            "switch",
            "synchronized",
            "this",
            "throw",
            "throws",
            "transient",
            "try",
            "void",
            "volatile",
            "while",
            "true",
            "false",
            "null",
            "var",
            "record",
            "sealed",
            "permits",
            "yield",
            "_",
        ];
    let mut out = String::new();
    for c in cased.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || out.chars().next().unwrap().is_ascii_digit() || reserved.contains(&out.as_str()) {
        out.insert(0, '_');
    }
    return out;
}

fn java_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    return out;
}

fn javadoc(desc: Option<&str>) -> String {
    return match desc {
        None => String::new(),
        Some(d) => format!("/** {} */\n", d.replace("*/", "* /")),
    };
}

fn exception(message: &str, path: &str) -> String {
    return format!("throw new IllegalArgumentException({} + \" at \" + {});", message, path);
}

fn field_path(path: &str, key: &str) -> String {
    return format!("{} + {}", path, java_string(&format!(".{}", key)));
}

struct CodeGen<'a> {
    schema: &'a SchemaskV1,
    config: &'a JavaConfig,
    files: Vec<JavaFile>,
    names: BTreeMap<String, String>,
    used: BTreeSet<String>,
    generated: BTreeSet<String>,
    counter: usize,
}

impl<'a> CodeGen<'a> {
    fn tmp(&mut self, base: &str) -> String {
        self.counter += 1;
        return format!("__{}{}", base, self.counter);
    }

    fn fresh(&mut self, hint: &str) -> String {
        let mut name = hint.to_string();
        let mut i = 2;
        while !self.used.insert(name.clone()) {
            name = format!("{}{}", hint, i);
            i += 1;
        }
        return name;
    }

    fn name_for(&mut self, maskoid: &Maskoid, hint: &str) -> String {
        let key = format!("{}\0{}", hint, serde_json::to_string(maskoid).unwrap());
        if let Some(name) = self.names.get(&key) {
            return name.clone();
        }
        let name = self.fresh(hint);
        self.names.insert(key, name.clone());
        return name;
    }

    fn type_of(&mut self, maskoid: &Maskoid, hint: &str, boxed: bool) -> String {
        return match maskoid {
            Maskoid::Null(_) => "Void".to_string(),
            Maskoid::String(_) | Maskoid::ConstString(_) => "String".to_string(),
            Maskoid::Bool(_) => if boxed {
                "Boolean".to_string()
            } else {
                "boolean".to_string()
            },
            Maskoid::Int(_) => if boxed {
                "Long".to_string()
            } else {
                "long".to_string()
            },
            Maskoid::Float(_) => if boxed {
                "Double".to_string()
            } else {
                "double".to_string()
            },
            Maskoid::Any(_) => JSON_NODE.to_string(),
            Maskoid::Ref(MaskoidRef { name, .. }) => {
                let target =
                    self.schema.bindings.get(name).unwrap_or_else(|| panic!("Unknown binding ref {}", name));
                if is_named(target) {
                    name.clone()
                } else {
                    self.type_of(target, name, boxed)
                }
            },
            Maskoid::Option(MaskoidElement { element: inner, .. }) => {
                let inner_ty = self.option_inner_type(inner, hint);
                format!("{}<{}>", OPTIONAL, inner_ty)
            },
            Maskoid::List(MaskoidElement { element: inner, .. }) => {
                let inner_ty = self.type_of(inner, hint, true);
                format!("java.util.List<{}>", inner_ty)
            },
            Maskoid::Set(MaskoidElement { element: inner, .. }) => {
                let inner_ty = self.type_of(inner, hint, true);
                format!("java.util.Set<{}>", inner_ty)
            },
            Maskoid::StringMap(MaskoidElement { element: inner, .. }) => {
                let inner_ty = self.type_of(inner, hint, true);
                format!("java.util.Map<String, {}>", inner_ty)
            },
            Maskoid::Tuple(_) | Maskoid::Record(_) | Maskoid::TaggedUnion(_) => self.ensure_named(maskoid, hint),
        };
    }

    fn option_inner_type(&mut self, inner: &Maskoid, hint: &str) -> String {
        if matches!(inner, Maskoid::Option(_)) {
            let wrapper = self.name_for(inner, &format!("{}Wrapper", hint));
            if self.generated.insert(wrapper.clone()) {
                let element_ty = self.type_of(inner, &format!("{}Inner", hint), true);
                self.files.push(JavaFile {
                    path: self.file_path(&wrapper),
                    source: format!("{}public record {}({} element) {{}}\n", self.header(), wrapper, element_ty),
                });
                let enc_out = self.tmp("j");
                let enc = self.encode(inner, &format!("{}Inner", hint), "value.element()", &enc_out);
                let dec_out = self.tmp("v");
                let dec =
                    self.decode(
                        inner,
                        &format!("{}Inner", hint),
                        "value.get(\"element\")",
                        &field_path("path", "element"),
                        &dec_out,
                    );
                let source =
                    format!(
                        "{header}public final class {w}Codec {{\n    private {w}Codec() {{}}\n\n    public static {jn} toValue({w} value) {{\n{enc}        {on} __o = {f}.objectNode();\n        __o.set(\"element\", {enc_out});\n        return __o;\n    }}\n\n    public static {w} fromValue({jn} value) {{\n        return fromValue(value, \"$\");\n    }}\n\n    public static {w} fromValue({jn} value, String path) {{\n        if (!value.isObject() || value.size() != 1 || !value.has(\"element\")) {{\n            {exc}\n        }}\n{dec}        return new {w}({dec_out});\n    }}\n}}\n",
                        header = self.header(),
                        w = wrapper,
                        jn = JSON_NODE,
                        on = OBJECT_NODE,
                        f = NODE_FACTORY,
                        enc = indent(&enc, 2),
                        enc_out = enc_out,
                        exc = exception("\"Expected an object with exactly the key 'element'\"", "path"),
                        dec = indent(&dec, 2),
                        dec_out = dec_out,
                    );
                self.files.push(JavaFile {
                    path: self.file_path(&format!("{}Codec", wrapper)),
                    source: source,
                });
            }
            return wrapper;
        }
        return self.type_of(inner, hint, true);
    }

    fn header(&self) -> String {
        return format!("package {};\n\n", self.config.package);
    }

    fn file_path(&self, name: &str) -> String {
        return format!("{}/{}.java", self.config.package.replace('.', "/"), name);
    }

    fn ensure_named(&mut self, maskoid: &Maskoid, hint: &str) -> String {
        let name = self.name_for(maskoid, hint);
        if !self.generated.insert(name.clone()) {
            return name;
        }
        let name = name.as_str();
        match maskoid {
            Maskoid::Record(r) => {
                let parts = self.record_parts(name, &r.fields, "value", "value", "path");
                self.files.push(JavaFile {
                    path: self.file_path(name),
                    source: format!(
                        "{}{}public record {}({}) {{}}\n",
                        self.header(),
                        javadoc(r.description.as_deref()),
                        name,
                        parts.components
                    ),
                });
                let source =
                    format!(
                        "{header}public final class {n}Codec {{\n    private {n}Codec() {{}}\n\n    public static {jn} toValue({n} value) {{\n{enc}        return __o;\n    }}\n\n    public static {n} fromValue({jn} value) {{\n        return fromValue(value, \"$\");\n    }}\n\n    public static {n} fromValue({jn} value, String path) {{\n{dec}        return new {n}({ctor});\n    }}\n{root}}}\n",
                        header = self.header(),
                        n = name,
                        jn = JSON_NODE,
                        enc = indent(&parts.encode, 2),
                        dec = indent(&parts.decode, 2),
                        ctor = parts.ctor,
                        root = self.root_methods(name, name),
                    );
                self.files.push(JavaFile {
                    path: self.file_path(&format!("{}Codec", name)),
                    source: source,
                });
            },
            Maskoid::TaggedUnion(u) => {
                let mut sorted: Vec<_> = u.variants.iter().collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                let mut permits = vec![];
                let mut enc_cases = String::new();
                let mut dec_string_cases = String::new();
                let mut dec_object_cases = String::new();
                for (tag, variant) in sorted {
                    let vname = self.fresh(&format!("{}{}", name, to_java_ident(tag.to_upper_camel_case())));
                    permits.push(vname.clone());
                    match &variant.maskoid {
                        Maskoid::Null(_) => {
                            self.files.push(JavaFile {
                                path: self.file_path(&vname),
                                source: format!(
                                    "{}{}public record {}() implements {} {{}}\n",
                                    self.header(),
                                    javadoc(variant.description.as_deref()),
                                    vname,
                                    name
                                ),
                            });
                            enc_cases.push_str(
                                &format!("case {} __v -> {}.textNode({});\n", vname, NODE_FACTORY, java_string(tag)),
                            );
                            dec_string_cases.push_str(&format!("case {} -> new {}();\n", java_string(tag), vname));
                            dec_object_cases.push_str(
                                &format!(
                                    "case {} -> {{\n    if (!__entry.getValue().isNull()) {{\n        {}\n    }}\n    yield new {}();\n}}\n",
                                    java_string(tag),
                                    exception("\"Expected null\"", "__p"),
                                    vname
                                ),
                            );
                        },
                        Maskoid::Record(r) => {
                            let parts = self.record_parts(&vname, &r.fields, "__v", "__entry.getValue()", "__p");
                            self.files.push(JavaFile {
                                path: self.file_path(&vname),
                                source: format!(
                                    "{}{}public record {}({}) implements {} {{}}\n",
                                    self.header(),
                                    javadoc(variant.description.as_deref()),
                                    vname,
                                    parts.components,
                                    name
                                ),
                            });
                            enc_cases.push_str(
                                &format!(
                                    "case {} __v -> {{\n{}    {on} __u = {f}.objectNode();\n    __u.set({}, __o);\n    yield __u;\n}}\n",
                                    vname,
                                    indent(&parts.encode, 1),
                                    java_string(tag),
                                    on = OBJECT_NODE,
                                    f = NODE_FACTORY,
                                ),
                            );
                            dec_object_cases.push_str(
                                &format!(
                                    "case {} -> {{\n{}    yield new {}({});\n}}\n",
                                    java_string(tag),
                                    indent(&parts.decode, 1),
                                    vname,
                                    parts.ctor
                                ),
                            );
                        },
                        other => {
                            let ty = self.type_of(other, &vname, false);
                            self.files.push(JavaFile {
                                path: self.file_path(&vname),
                                source: format!(
                                    "{}{}public record {}({} value) implements {} {{}}\n",
                                    self.header(),
                                    javadoc(variant.description.as_deref()),
                                    vname,
                                    ty,
                                    name
                                ),
                            });
                            let out = self.tmp("j");
                            let body = self.encode(other, &vname, "__v.value()", &out);
                            enc_cases.push_str(
                                &format!(
                                    "case {} __v -> {{\n{}    {on} __o = {f}.objectNode();\n    __o.set({}, {});\n    yield __o;\n}}\n",
                                    vname,
                                    indent(&body, 1),
                                    java_string(tag),
                                    out,
                                    on = OBJECT_NODE,
                                    f = NODE_FACTORY,
                                ),
                            );
                            let v = self.tmp("v");
                            let body = self.decode(other, &vname, "__entry.getValue()", "__p", &v);
                            dec_object_cases.push_str(
                                &format!(
                                    "case {} -> {{\n{}    yield new {}({});\n}}\n",
                                    java_string(tag),
                                    indent(&body, 1),
                                    vname,
                                    v
                                ),
                            );
                        },
                    }
                }
                self.files.push(JavaFile {
                    path: self.file_path(name),
                    source: format!(
                        "{}{}public sealed interface {} permits {} {{}}\n",
                        self.header(),
                        javadoc(u.description.as_deref()),
                        name,
                        permits.join(", ")
                    ),
                });
                let unknown_tag =
                    exception(&format!("\"Unknown variant '\" + value.textValue() + \"' of {}\"", name), "path");
                let string_case = if dec_string_cases.is_empty() {
                    format!("            {}\n", unknown_tag)
                } else {
                    format!(
                        "            return switch (value.textValue()) {{\n{}                default -> {{\n                    {}\n                }}\n            }};\n",
                        indent(&dec_string_cases, 4),
                        unknown_tag
                    )
                };
                let source =
                    format!(
                        "{header}public final class {n}Codec {{\n    private {n}Codec() {{}}\n\n    public static {jn} toValue({n} value) {{\n        return switch (value) {{\n{enc}        }};\n    }}\n\n    public static {n} fromValue({jn} value) {{\n        return fromValue(value, \"$\");\n    }}\n\n    public static {n} fromValue({jn} value, String path) {{\n        if (value.isTextual()) {{\n{string_case}        }}\n        if (!value.isObject() || value.size() != 1) {{\n            {one_key}\n        }}\n        var __entry = value.properties().iterator().next();\n        String __p = path + \".\" + __entry.getKey();\n        return switch (__entry.getKey()) {{\n{dec_object}            default -> {{\n                {unknown_variant}\n            }}\n        }};\n    }}\n{root}}}\n",
                        header = self.header(),
                        n = name,
                        jn = JSON_NODE,
                        enc = indent(&enc_cases, 3),
                        string_case = string_case,
                        one_key =
                            exception(
                                "\"Expected an object with exactly one key, or a string for a valueless variant\"",
                                "path",
                            ),
                        dec_object = indent(&dec_object_cases, 3),
                        unknown_variant =
                            exception(&format!("\"Unknown variant '\" + __entry.getKey() + \"' of {}\"", name), "__p"),
                        root = self.root_methods(name, name),
                    );
                self.files.push(JavaFile {
                    path: self.file_path(&format!("{}Codec", name)),
                    source: source,
                });
            },
            Maskoid::Tuple(t) => {
                let mut components = vec![];
                let mut enc = String::new();
                let mut enc_outs = vec![];
                let mut dec = String::new();
                let mut dec_outs = vec![];
                for (i, element) in t.elements.iter().enumerate() {
                    let hint = format!("{}_{}", name, i);
                    let ty = self.type_of(&element.maskoid, &hint, false);
                    components.push(format!("{}{} e{}", javadoc(element.description.as_deref()), ty, i));
                    let out = self.tmp("j");
                    enc.push_str(&self.encode(&element.maskoid, &hint, &format!("value.e{}()", i), &out));
                    enc_outs.push(out);
                    let v = self.tmp("v");
                    dec.push_str(
                        &self.decode(
                            &element.maskoid,
                            &hint,
                            &format!("value.get({})", i),
                            &format!("path + \"[{}]\"", i),
                            &v,
                        ),
                    );
                    dec_outs.push(v);
                }
                self.files.push(JavaFile {
                    path: self.file_path(name),
                    source: format!(
                        "{}{}public record {}({}) {{}}\n",
                        self.header(),
                        javadoc(t.description.as_deref()),
                        name,
                        components.join(", ")
                    ),
                });
                let adds: String = enc_outs.iter().map(|o| format!("        __a.add({});\n", o)).collect();
                let source =
                    format!(
                        "{header}public final class {n}Codec {{\n    private {n}Codec() {{}}\n\n    public static {jn} toValue({n} value) {{\n{enc}        {an} __a = {f}.arrayNode();\n{adds}        return __a;\n    }}\n\n    public static {n} fromValue({jn} value) {{\n        return fromValue(value, \"$\");\n    }}\n\n    public static {n} fromValue({jn} value, String path) {{\n        if (!value.isArray() || value.size() != {len}) {{\n            {bad_len}\n        }}\n{dec}        return new {n}({dec_outs});\n    }}\n{root}}}\n",
                        header = self.header(),
                        n = name,
                        jn = JSON_NODE,
                        an = ARRAY_NODE,
                        f = NODE_FACTORY,
                        enc = indent(&enc, 2),
                        adds = adds,
                        len = t.elements.len(),
                        bad_len = exception(&format!("\"Expected an array of length {}\"", t.elements.len()), "path"),
                        dec = indent(&dec, 2),
                        dec_outs = dec_outs.join(", "),
                        root = self.root_methods(name, name),
                    );
                self.files.push(JavaFile {
                    path: self.file_path(&format!("{}Codec", name)),
                    source: source,
                });
            },
            _ => unreachable!(),
        }
        return name.to_string();
    }

    fn record_parts(
        &mut self,
        name: &str,
        fields: &BTreeMap<String, MaskoidField>,
        receiver: &str,
        json: &str,
        path: &str,
    ) -> RecordParts {
        let mut sorted: Vec<_> = fields.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        let mut components = vec![];
        let mut enc = String::new();
        let mut dec_locals = String::new();
        let mut dec_cases = String::new();
        let mut dec_checks = String::new();
        let mut ctor_args = vec![];
        for (key, field) in sorted {
            let ident = to_java_ident(key.to_lower_camel_case());
            let hint = format!("{}{}", name, to_java_ident(key.to_upper_camel_case()));
            let ty = self.type_of(&field.maskoid, &hint, false);
            components.push(format!("{}{} {}", javadoc(field.description.as_deref()), ty, ident));
            let out = self.tmp("j");
            let v = self.tmp("v");
            match &field.maskoid {
                Maskoid::Option(MaskoidElement { element: inner, .. }) => {
                    let unwrapped = format!("{}.{}().get()", receiver, ident);
                    let body = self.encode_option_inner(inner, &hint, &unwrapped, &out);
                    enc.push_str(
                        &format!(
                            "if ({}.{}().isPresent()) {{\n{}    __o.set({}, {});\n}}\n",
                            receiver,
                            ident,
                            indent(&body, 1),
                            java_string(key),
                            out
                        ),
                    );
                    dec_locals.push_str(&format!("{} __f_{} = {}.empty();\n", ty, ident, OPTIONAL));
                    let body = self.decode(&field.maskoid, &hint, "__fe.getValue()", "__fp", &v);
                    dec_cases.push_str(
                        &format!(
                            "case {} -> {{\n    String __fp = {};\n{}    __f_{} = {};\n}}\n",
                            java_string(key),
                            field_path(path, key),
                            indent(&body, 1),
                            ident,
                            v
                        ),
                    );
                    ctor_args.push(format!("__f_{}", ident));
                },
                other => {
                    let body = self.encode(other, &hint, &format!("{}.{}()", receiver, ident), &out);
                    enc.push_str(&format!("{}__o.set({}, {});\n", body, java_string(key), out));
                    let body = self.decode(other, &hint, "__fe.getValue()", "__fp", &v);
                    if matches!(other, Maskoid::Null(_)) {
                        dec_locals.push_str(&format!("boolean __seen_{} = false;\n", ident));
                        dec_cases.push_str(
                            &format!(
                                "case {} -> {{\n    String __fp = {};\n{}    __seen_{} = true;\n}}\n",
                                java_string(key),
                                field_path(path, key),
                                indent(&body, 1),
                                ident
                            ),
                        );
                        dec_checks.push_str(
                            &format!(
                                "if (!__seen_{}) {{\n    {}\n}}\n",
                                ident,
                                exception(
                                    &format!("\"Missing required field {}\"", java_string(key).replace('"', "'")),
                                    &field_path(path, key),
                                )
                            ),
                        );
                        ctor_args.push("null".to_string());
                    } else {
                        let boxed_ty = self.type_of(other, &hint, true);
                        dec_locals.push_str(
                            &format!("{}<{}> __f_{} = {}.empty();\n", OPTIONAL, boxed_ty, ident, OPTIONAL),
                        );
                        dec_cases.push_str(
                            &format!(
                                "case {} -> {{\n    String __fp = {};\n{}    __f_{} = {}.of({});\n}}\n",
                                java_string(key),
                                field_path(path, key),
                                indent(&body, 1),
                                ident,
                                OPTIONAL,
                                v
                            ),
                        );
                        dec_checks.push_str(
                            &format!(
                                "if (__f_{}.isEmpty()) {{\n    {}\n}}\n",
                                ident,
                                exception(
                                    &format!("\"Missing required field {}\"", java_string(key).replace('"', "'")),
                                    &field_path(path, key),
                                )
                            ),
                        );
                        ctor_args.push(format!("__f_{}.get()", ident));
                    }
                },
            }
        }
        let decode =
            format!(
                "if (!{json}.isObject()) {{\n    {not_object}\n}}\n{locals}for (var __fe : {json}.properties()) {{\n    switch (__fe.getKey()) {{\n{cases}        default -> {{\n            {unknown}\n        }}\n    }}\n}}\n{checks}",
                json = json,
                not_object = exception("\"Expected an object\"", path),
                locals = dec_locals,
                cases = indent(&dec_cases, 2),
                unknown =
                    exception(
                        &format!("\"Unknown field '\" + __fe.getKey() + \"' in {}\"", name),
                        &format!("{} + \".\" + __fe.getKey()", path),
                    ),
                checks = dec_checks,
            );
        return RecordParts {
            components: components.join(", "),
            encode: format!("{} __o = {}.objectNode();\n{}", OBJECT_NODE, NODE_FACTORY, enc),
            decode: decode,
            ctor: ctor_args.join(", "),
        };
    }

    fn root_methods(&self, name: &str, ty: &str) -> String {
        if self.schema.default.as_deref() != Some(name) {
            return String::new();
        }
        return format!(
            "\n    private static final com.fasterxml.jackson.databind.ObjectMapper MAPPER =\n        new com.fasterxml.jackson.databind.ObjectMapper()\n            .enable(com.fasterxml.jackson.databind.DeserializationFeature.FAIL_ON_TRAILING_TOKENS);\n\n    public static String serialize({n} value) throws com.fasterxml.jackson.core.JsonProcessingException {{\n        return MAPPER.writeValueAsString(toValue(value));\n    }}\n\n    public static {n} deserialize(String json) throws com.fasterxml.jackson.core.JsonProcessingException {{\n        return fromValue(MAPPER.readTree(json), \"$\");\n    }}\n",
            n = ty,
        );
    }

    fn encode(&mut self, maskoid: &Maskoid, hint: &str, expr: &str, out: &str) -> String {
        return match maskoid {
            Maskoid::Null(_) => format!("{} {} = {}.nullNode();\n", JSON_NODE, out, NODE_FACTORY),
            Maskoid::String(_) | Maskoid::ConstString(_) => format!(
                "{} {} = {}.textNode({});\n",
                JSON_NODE,
                out,
                NODE_FACTORY,
                expr
            ),
            Maskoid::Bool(_) => format!("{} {} = {}.booleanNode({});\n", JSON_NODE, out, NODE_FACTORY, expr),
            Maskoid::Int(_) | Maskoid::Float(_) => format!(
                "{} {} = {}.numberNode({});\n",
                JSON_NODE,
                out,
                NODE_FACTORY,
                expr
            ),
            Maskoid::Any(_) => format!("{} {} = {};\n", JSON_NODE, out, expr),
            Maskoid::Ref(MaskoidRef { name, .. }) => format!(
                "{} {} = {}Codec.toValue({});\n",
                JSON_NODE,
                out,
                name,
                expr
            ),
            Maskoid::Option(MaskoidElement { element: inner, .. }) => {
                let unwrapped = format!("{}.get()", expr);
                let some = self.tmp("j");
                let body = self.encode_option_inner(inner, hint, &unwrapped, &some);
                format!(
                    "{jn} {out};\nif ({expr}.isPresent()) {{\n{body}    {out} = {some};\n}} else {{\n    {out} = {f}.nullNode();\n}}\n",
                    jn = JSON_NODE,
                    out = out,
                    expr = expr,
                    body = indent(&body, 1),
                    some = some,
                    f = NODE_FACTORY,
                )
            },
            Maskoid::Set(MaskoidElement { element: inner, .. }) |
            Maskoid::List(MaskoidElement { element: inner, .. }) => {
                let list = self.tmp("a");
                let element = self.tmp("e");
                let element_out = self.tmp("j");
                let body = self.encode(inner, hint, &element, &element_out);
                format!(
                    "{an} {list} = {f}.arrayNode();\nfor (var {element} : {expr}) {{\n{body}    {list}.add({element_out});\n}}\n{jn} {out} = {list};\n",
                    an = ARRAY_NODE,
                    list = list,
                    f = NODE_FACTORY,
                    element = element,
                    expr = expr,
                    body = indent(&body, 1),
                    element_out = element_out,
                    jn = JSON_NODE,
                    out = out,
                )
            },
            Maskoid::StringMap(MaskoidElement { element: inner, .. }) => {
                let map = self.tmp("m");
                let entry = self.tmp("e");
                let element_out = self.tmp("j");
                let body = self.encode(inner, hint, &format!("{}.getValue()", entry), &element_out);
                format!(
                    "{on} {map} = {f}.objectNode();\nfor (var {entry} : {expr}.entrySet()) {{\n{body}    {map}.set({entry}.getKey(), {element_out});\n}}\n{jn} {out} = {map};\n",
                    on = OBJECT_NODE,
                    map = map,
                    f = NODE_FACTORY,
                    entry = entry,
                    expr = expr,
                    body = indent(&body, 1),
                    element_out = element_out,
                    jn = JSON_NODE,
                    out = out,
                )
            },
            Maskoid::Tuple(_) | Maskoid::Record(_) | Maskoid::TaggedUnion(_) => {
                let name = self.ensure_named(maskoid, hint);
                format!("{} {} = {}Codec.toValue({});\n", JSON_NODE, out, name, expr)
            },
        };
    }

    fn encode_option_inner(&mut self, inner: &Maskoid, hint: &str, expr: &str, out: &str) -> String {
        if matches!(inner, Maskoid::Option(_)) {
            let wrapper = self.option_inner_type(inner, hint);
            return format!("{} {} = {}Codec.toValue({});\n", JSON_NODE, out, wrapper, expr);
        }
        return self.encode(inner, hint, expr, out);
    }

    fn decode(&mut self, maskoid: &Maskoid, hint: &str, json: &str, path: &str, out: &str) -> String {
        let ty = self.type_of(maskoid, hint, false);
        return match maskoid {
            Maskoid::Null(_) => format!(
                "if (!{json}.isNull()) {{\n    {exc}\n}}\n{ty} {out} = null;\n",
                json = json,
                exc = exception("\"Expected null\"", path),
                ty = ty,
                out = out,
            ),
            Maskoid::String(_) => format!(
                "if (!{json}.isTextual()) {{\n    {exc}\n}}\n{ty} {out} = {json}.textValue();\n",
                json = json,
                exc = exception("\"Expected a string\"", path),
                ty = ty,
                out = out,
            ),
            Maskoid::ConstString(MaskoidConstString { value: expected, .. }) => format!(
                "if (!{json}.isTextual() || !{json}.textValue().equals({expected})) {{\n    {exc}\n}}\n{ty} {out} = {json}.textValue();\n",
                json = json,
                expected = java_string(expected),
                exc = exception(&format!("\"Expected the string \" + {}", java_string(expected)), path),
                ty = ty,
                out = out,
            ),
            Maskoid::Bool(_) => format!(
                "if (!{json}.isBoolean()) {{\n    {exc}\n}}\n{ty} {out} = {json}.booleanValue();\n",
                json = json,
                exc = exception("\"Expected a boolean\"", path),
                ty = ty,
                out = out,
            ),
            Maskoid::Int(_) => format!(
                "if (!{json}.isIntegralNumber() || !{json}.canConvertToLong()) {{\n    {exc}\n}}\n{ty} {out} = {json}.longValue();\n",
                json = json,
                exc = exception("\"Expected an integer\"", path),
                ty = ty,
                out = out,
            ),
            Maskoid::Float(_) => format!(
                "if (!{json}.isNumber()) {{\n    {exc}\n}}\n{ty} {out} = {json}.doubleValue();\n",
                json = json,
                exc = exception("\"Expected a number\"", path),
                ty = ty,
                out = out,
            ),
            Maskoid::Any(_) => format!("{} {} = {};\n", ty, out, json),
            Maskoid::Ref(MaskoidRef { name, .. }) => format!(
                "{} {} = {}Codec.fromValue({}, {});\n",
                ty,
                out,
                name,
                json,
                path
            ),
            Maskoid::Option(MaskoidElement { element: inner, .. }) => {
                let v = self.tmp("v");
                let body = if matches!(inner.as_ref(), Maskoid::Option(_)) {
                    let wrapper = self.option_inner_type(inner, hint);
                    format!("{} {} = {}Codec.fromValue({}, {});\n", wrapper, v, wrapper, json, path)
                } else {
                    self.decode(inner, hint, json, path, &v)
                };
                let wrap = if matches!(inner.as_ref(), Maskoid::Null(_)) {
                    "ofNullable"
                } else {
                    "of"
                };
                format!(
                    "{ty} {out};\nif ({json}.isNull()) {{\n    {out} = {opt}.empty();\n}} else {{\n{body}    {out} = {opt}.{wrap}({v});\n}}\n",
                    ty = ty,
                    out = out,
                    json = json,
                    opt = OPTIONAL,
                    body = indent(&body, 1),
                    wrap = wrap,
                    v = v,
                )
            },
            Maskoid::Set(MaskoidElement { element: inner, .. }) |
            Maskoid::List(MaskoidElement { element: inner, .. }) => {
                let index = self.tmp("i");
                let element = self.tmp("e");
                let v = self.tmp("v");
                let body = self.decode(inner, hint, &element, &format!("{} + \"[\" + {} + \"]\"", path, index), &v);
                let (collection, add) = match maskoid {
                    Maskoid::Set(_) => (
                        "java.util.LinkedHashSet",
                        format!(
                            "    if (!{out}.add({v})) {{\n        {exc}\n    }}\n",
                            out = out,
                            v = v,
                            exc =
                                exception(
                                    "\"Duplicate element in set\"",
                                    &format!("{} + \"[\" + {} + \"]\"", path, index)
                                ),
                        ),
                    ),
                    _ => ("java.util.ArrayList", format!("    {}.add({});\n", out, v)),
                };
                format!(
                    "if (!{json}.isArray()) {{\n    {exc}\n}}\n{ty} {out} = new {collection}<>();\nint {index} = 0;\nfor (var {element} : {json}) {{\n{body}{add}    {index}++;\n}}\n",
                    json = json,
                    exc = exception("\"Expected an array\"", path),
                    ty = ty,
                    out = out,
                    collection = collection,
                    index = index,
                    element = element,
                    body = indent(&body, 1),
                    add = add,
                )
            },
            Maskoid::StringMap(MaskoidElement { element: inner, .. }) => {
                let entry = self.tmp("e");
                let v = self.tmp("v");
                let body =
                    self.decode(
                        inner,
                        hint,
                        &format!("{}.getValue()", entry),
                        &format!("{} + \".\" + {}.getKey()", path, entry),
                        &v,
                    );
                format!(
                    "if (!{json}.isObject()) {{\n    {exc}\n}}\n{ty} {out} = new java.util.LinkedHashMap<>();\nfor (var {entry} : {json}.properties()) {{\n{body}    {out}.put({entry}.getKey(), {v});\n}}\n",
                    json = json,
                    exc = exception("\"Expected an object\"", path),
                    ty = ty,
                    out = out,
                    entry = entry,
                    body = indent(&body, 1),
                    v = v,
                )
            },
            Maskoid::Tuple(_) | Maskoid::Record(_) | Maskoid::TaggedUnion(_) => {
                let name = self.ensure_named(maskoid, hint);
                format!("{} {} = {}Codec.fromValue({}, {});\n", ty, out, name, json, path)
            },
        };
    }
}

struct RecordParts {
    components: String,
    encode: String,
    decode: String,
    ctor: String,
}

fn is_named(maskoid: &Maskoid) -> bool {
    return matches!(maskoid, Maskoid::Record(_) | Maskoid::TaggedUnion(_) | Maskoid::Tuple(_));
}

fn indent(text: &str, levels: usize) -> String {
    let pad = "    ".repeat(levels);
    let mut out = String::new();
    for line in text.lines() {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&pad);
            out.push_str(line);
            out.push('\n');
        }
    }
    return out;
}

pub fn generate_java(schema: &SchemaskV1, config: &JavaConfig) -> Result<Vec<JavaFile>, crate::Invalid> {
    crate::v1::check_schema(schema)?;
    let mut codegen = CodeGen {
        schema: schema,
        config: config,
        files: vec![],
        names: BTreeMap::new(),
        used: BTreeSet::new(),
        generated: BTreeSet::new(),
        counter: 0,
    };
    for (name, maskoid) in &schema.bindings {
        codegen.used.insert(name.clone());
        if is_named(maskoid) {
            let key = format!("{}\0{}", name, serde_json::to_string(maskoid).unwrap());
            codegen.names.insert(key, name.clone());
        }
    }
    let mut sorted_bindings: Vec<_> = schema.bindings.iter().collect();
    sorted_bindings.sort_by_key(|(k, _)| k.as_str());
    for (name, maskoid) in sorted_bindings {
        if is_named(maskoid) {
            codegen.ensure_named(maskoid, name);
        } else {
            let ty = codegen.type_of(maskoid, name, false);
            let out = codegen.tmp("j");
            let enc = codegen.encode(maskoid, name, "value", &out);
            let v = codegen.tmp("v");
            let dec = codegen.decode(maskoid, name, "value", "path", &v);
            let source =
                format!(
                    "{header}{doc}public final class {n}Codec {{\n    private {n}Codec() {{}}\n\n    public static {jn} toValue({ty} value) {{\n{enc}        return {out};\n    }}\n\n    public static {ty} fromValue({jn} value) {{\n        return fromValue(value, \"$\");\n    }}\n\n    public static {ty} fromValue({jn} value, String path) {{\n{dec}        return {v};\n    }}\n{root}}}\n",
                    header = codegen.header(),
                    doc = javadoc(maskoid.description()),
                    n = name,
                    jn = JSON_NODE,
                    ty = ty,
                    enc = indent(&enc, 2),
                    out = out,
                    dec = indent(&dec, 2),
                    v = v,
                    root = codegen.root_methods(name, &ty),
                );
            codegen.files.push(JavaFile {
                path: codegen.file_path(&format!("{}Codec", name)),
                source: source,
            });
        }
    }
    codegen.files.sort_by(|a, b| a.path.cmp(&b.path));
    return Ok(codegen.files);
}
