use {
    crate::{
        Maskoid,
        MaskoidField,
        v1::SchemaskV1,
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
    pub runtime_package: String,
    pub prelude_package: String,
}

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

struct CodeGen<'a> {
    schema: &'a SchemaskV1,
    config: &'a JavaConfig,
    files: Vec<JavaFile>,
    generated: BTreeSet<String>,
    counter: usize,
}

impl<'a> CodeGen<'a> {
    fn tmp(&mut self, base: &str) -> String {
        self.counter += 1;
        return format!("__{}{}", base, self.counter);
    }

    fn rt(&self, name: &str) -> String {
        return format!("{}.{}", self.config.runtime_package, name);
    }

    fn pre(&self, name: &str) -> String {
        return format!("{}.{}", self.config.prelude_package, name);
    }

    fn exception(&self, message: &str, path: &str) -> String {
        return format!("throw new {}.DeserializationException({}, {});", self.rt("Json"), message, path);
    }

    fn type_of(&mut self, maskoid: &Maskoid, hint: &str, boxed: bool) -> String {
        return match maskoid {
            Maskoid::Null => self.rt("JsonValueNull"),
            Maskoid::String | Maskoid::ConstString(_) => "String".to_string(),
            Maskoid::Bool => if boxed {
                "Boolean".to_string()
            } else {
                "boolean".to_string()
            },
            Maskoid::Int => if boxed {
                "Long".to_string()
            } else {
                "long".to_string()
            },
            Maskoid::Float => if boxed {
                "Double".to_string()
            } else {
                "double".to_string()
            },
            Maskoid::Any => self.rt("JsonValue"),
            Maskoid::Ref(name) => {
                let target =
                    self.schema.bindings.get(name).unwrap_or_else(|| panic!("Unknown binding ref {}", name));
                if is_named(target) {
                    name.clone()
                } else {
                    self.type_of(target, name, boxed)
                }
            },
            Maskoid::Option(inner) => {
                let inner_ty = self.option_inner_type(inner, hint);
                format!("{}<{}>", self.pre("Opt"), inner_ty)
            },
            Maskoid::Set(inner) | Maskoid::List(inner) => {
                let inner_ty = self.type_of(inner, hint, true);
                format!("{}<{}>", self.pre("TList"), inner_ty)
            },
            Maskoid::StringMap(inner) => {
                let inner_ty = self.type_of(inner, hint, true);
                format!("{}<String, {}>", self.pre("TOrderedMap"), inner_ty)
            },
            Maskoid::Tuple(_) | Maskoid::Record(_) | Maskoid::TaggedUnion(_) => {
                self.ensure_named(maskoid, hint);
                hint.to_string()
            },
        };
    }

    fn option_inner_type(&mut self, inner: &Maskoid, hint: &str) -> String {
        if matches!(inner, Maskoid::Option(_)) {
            let wrapper = format!("{}Wrapper", hint);
            if self.generated.insert(wrapper.clone()) {
                let element_ty = self.type_of(inner, &format!("{}Inner", hint), true);
                self.files.push(JavaFile {
                    path: self.file_path(&wrapper),
                    source: format!("{}public record {}({} element) {{}}\n", self.header(), wrapper, element_ty),
                });
                let mut enc = String::new();
                let enc_out = self.tmp("j");
                enc.push_str(&self.encode(inner, &format!("{}Inner", hint), "value.element()", &enc_out));
                let mut dec = String::new();
                let dec_out = self.tmp("v");
                dec.push_str(
                    &self.decode(
                        inner,
                        &format!("{}Inner", hint),
                        "__entry.getValue()",
                        "path.field(\"element\")",
                        &dec_out,
                    ),
                );
                let source =
                    format!(
                        "{header}public final class {w}Codec {{\n    private {w}Codec() {{}}\n\n    public static {jv} toValue({w} value) {{\n{enc}        {om}<String, {jv}> __o = new {om}<>();\n        __o.put(\"element\", {enc_out});\n        return new {jo}(__o);\n    }}\n\n    public static {w} fromValue({jv} value, {json}.FieldPath path) {{\n        var __o = {jvs}.object(value, path);\n        if (__o.size() != 1 || !__o.containsKey(\"element\")) {{\n            {exc}\n        }}\n        var __entry = __o.entrySet().iterator().next();\n{dec}        return new {w}({dec_out});\n    }}\n}}\n",
                        header = self.header(),
                        w = wrapper,
                        jv = self.rt("JsonValue"),
                        jo = self.rt("JsonValueObject"),
                        jvs = self.rt("JsonValues"),
                        json = self.rt("Json"),
                        om = self.pre("TOrderedMap"),
                        enc = indent(&enc, 2),
                        enc_out = enc_out,
                        exc = self.exception("\"Expected an object with exactly the key 'element'\"", "path"),
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

    fn ensure_named(&mut self, maskoid: &Maskoid, name: &str) {
        if !self.generated.insert(name.to_string()) {
            return;
        }
        match maskoid {
            Maskoid::Record(r) => {
                let fields = &r.fields;
                let description = r.description.as_deref();
                let parts = self.record_parts(name, fields, "value", "value", "path");
                self.files.push(JavaFile {
                    path: self.file_path(name),
                    source: format!(
                        "{}{}public record {}({}) {{}}\n",
                        self.header(),
                        javadoc(description),
                        name,
                        parts.components
                    ),
                });
                let source =
                    format!(
                        "{header}public final class {n}Codec {{\n    private {n}Codec() {{}}\n\n    public static {jv} toValue({n} value) {{\n{enc}        return new {jo}(__o);\n    }}\n\n    public static {n} fromValue({jv} value, {json}.FieldPath path) {{\n{dec}        return new {n}({ctor});\n    }}\n{root}}}\n",
                        header = self.header(),
                        n = name,
                        jv = self.rt("JsonValue"),
                        jo = self.rt("JsonValueObject"),
                        json = self.rt("Json"),
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
                let variants = &u.variants;
                let description = u.description.as_deref();
                let mut sorted: Vec<_> = variants.iter().collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                let mut permits = vec![];
                let mut enc_cases = String::new();
                let mut dec_string_cases = String::new();
                let mut dec_object_cases = String::new();
                for (tag, variant) in sorted {
                    let vname = format!("{}{}", name, to_java_ident(tag.to_upper_camel_case()));
                    permits.push(vname.clone());
                    match &variant.maskoid {
                        Maskoid::Null => {
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
                                &format!(
                                    "case {} __v -> new {}({});\n",
                                    vname,
                                    self.rt("JsonValueString"),
                                    java_string(tag)
                                ),
                            );
                            dec_string_cases.push_str(&format!("case {} -> new {}();\n", java_string(tag), vname));
                            dec_object_cases.push_str(
                                &format!(
                                    "case {} -> {{\n    if (!(__entry.getValue() instanceof {})) {{\n        {}\n    }}\n    yield new {}();\n}}\n",
                                    java_string(tag),
                                    self.rt("JsonValueNull"),
                                    self.exception("\"Expected null\"", "__p"),
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
                                    "case {} __v -> {{\n{}    {om}<String, {jv}> __u = new {om}<>();\n    __u.put({}, new {}(__o));\n    yield new {}(__u);\n}}\n",
                                    vname,
                                    indent(&parts.encode, 1),
                                    java_string(tag),
                                    self.rt("JsonValueObject"),
                                    self.rt("JsonValueObject"),
                                    om = self.pre("TOrderedMap"),
                                    jv = self.rt("JsonValue"),
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
                                    "case {} __v -> {{\n{}    {om}<String, {jv}> __o = new {om}<>();\n    __o.put({}, {});\n    yield new {}(__o);\n}}\n",
                                    vname,
                                    indent(&body, 1),
                                    java_string(tag),
                                    out,
                                    self.rt("JsonValueObject"),
                                    om = self.pre("TOrderedMap"),
                                    jv = self.rt("JsonValue"),
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
                        javadoc(description),
                        name,
                        permits.join(", ")
                    ),
                });
                let unknown_tag =
                    self.exception(&format!("\"Unknown variant '\" + __s.value() + \"' of {}\"", name), "path");
                let string_case = if dec_string_cases.is_empty() {
                    format!("            {}\n", unknown_tag)
                } else {
                    format!(
                        "            return switch (__s.value()) {{\n{}                default -> {{\n                    {}\n                }}\n            }};\n",
                        indent(&dec_string_cases, 4),
                        unknown_tag
                    )
                };
                let source =
                    format!(
                        "{header}public final class {n}Codec {{\n    private {n}Codec() {{}}\n\n    public static {jv} toValue({n} value) {{\n        return switch (value) {{\n{enc}        }};\n    }}\n\n    public static {n} fromValue({jv} value, {json}.FieldPath path) {{\n        if (value instanceof {js} __s) {{\n{string_case}        }}\n        var __o = {jvs}.object(value, path);\n        if (__o.size() != 1) {{\n            {one_key}\n        }}\n        var __entry = __o.entrySet().iterator().next();\n        {json}.FieldPath __p = path.field(__entry.getKey());\n        return switch (__entry.getKey()) {{\n{dec_object}            default -> {{\n                {unknown_variant}\n            }}\n        }};\n    }}\n{root}}}\n",
                        header = self.header(),
                        n = name,
                        jv = self.rt("JsonValue"),
                        js = self.rt("JsonValueString"),
                        jvs = self.rt("JsonValues"),
                        json = self.rt("Json"),
                        enc = indent(&enc_cases, 3),
                        string_case = string_case,
                        one_key = self.exception("\"Expected an object with exactly one key\"", "path"),
                        dec_object = indent(&dec_object_cases, 3),
                        unknown_variant =
                            self.exception(
                                &format!("\"Unknown variant '\" + __entry.getKey() + \"' of {}\"", name),
                                "__p",
                            ),
                        root = self.root_methods(name, name),
                    );
                self.files.push(JavaFile {
                    path: self.file_path(&format!("{}Codec", name)),
                    source: source,
                });
            },
            Maskoid::Tuple(t) => {
                let elements = &t.elements;
                let description = t.description.as_deref();
                let mut components = vec![];
                let mut enc = String::new();
                let mut enc_outs = vec![];
                let mut dec = String::new();
                let mut dec_outs = vec![];
                for (i, element) in elements.iter().enumerate() {
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
                            &format!("__a.get({})", i),
                            &format!("path.index({})", i),
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
                        javadoc(description),
                        name,
                        components.join(", ")
                    ),
                });
                let source =
                    format!(
                        "{header}public final class {n}Codec {{\n    private {n}Codec() {{}}\n\n    public static {jv} toValue({n} value) {{\n{enc}        return new {ja}({tl}.of({outs}));\n    }}\n\n    public static {n} fromValue({jv} value, {json}.FieldPath path) {{\n        var __a = {jvs}.array(value, path);\n        if (__a.size() != {len}) {{\n            {bad_len}\n        }}\n{dec}        return new {n}({dec_outs});\n    }}\n{root}}}\n",
                        header = self.header(),
                        n = name,
                        jv = self.rt("JsonValue"),
                        ja = self.rt("JsonValueArray"),
                        jvs = self.rt("JsonValues"),
                        json = self.rt("Json"),
                        tl = self.pre("TList"),
                        enc = indent(&enc, 2),
                        outs = enc_outs.join(", "),
                        len = elements.len(),
                        bad_len =
                            self.exception(&format!("\"Expected an array of length {}\"", elements.len()), "path"),
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
            match &field.maskoid {
                Maskoid::Option(inner) => {
                    let unwrapped = format!("{}.{}().optUnwrap()", receiver, ident);
                    let body = self.encode_option_inner(inner, &hint, &unwrapped, &out);
                    enc.push_str(
                        &format!(
                            "if ({}.{}().optIsSome()) {{\n{}    __o.put({}, {});\n}}\n",
                            receiver,
                            ident,
                            indent(&body, 1),
                            java_string(key),
                            out
                        ),
                    );
                    dec_locals.push_str(&format!("{} __f_{} = {}.none();\n", ty, ident, self.pre("Opt")));
                    let v = self.tmp("v");
                    let body = self.decode(&field.maskoid, &hint, "__fe.getValue()", "__fp", &v);
                    dec_cases.push_str(
                        &format!(
                            "case {} -> {{\n    {}.FieldPath __fp = {}.field({});\n{}    __f_{} = {};\n}}\n",
                            java_string(key),
                            self.rt("Json"),
                            path,
                            java_string(key),
                            indent(&body, 1),
                            ident,
                            v
                        ),
                    );
                    ctor_args.push(format!("__f_{}", ident));
                },
                other => {
                    let body = self.encode(other, &hint, &format!("{}.{}()", receiver, ident), &out);
                    enc.push_str(&format!("{}__o.put({}, {});\n", body, java_string(key), out));
                    let boxed_ty = self.type_of(other, &hint, true);
                    dec_locals.push_str(
                        &format!("{}<{}> __f_{} = {}.none();\n", self.pre("Opt"), boxed_ty, ident, self.pre("Opt")),
                    );
                    let v = self.tmp("v");
                    let body = self.decode(other, &hint, "__fe.getValue()", "__fp", &v);
                    dec_cases.push_str(
                        &format!(
                            "case {} -> {{\n    {}.FieldPath __fp = {}.field({});\n{}    __f_{} = {}.some({});\n}}\n",
                            java_string(key),
                            self.rt("Json"),
                            path,
                            java_string(key),
                            indent(&body, 1),
                            ident,
                            self.pre("Opt"),
                            v
                        ),
                    );
                    dec_checks.push_str(
                        &format!(
                            "if (__f_{}.optIsNone()) {{\n    {}\n}}\n",
                            ident,
                            self.exception(
                                &format!("\"Missing required field {}\"", java_string(key).replace('"', "'")),
                                &format!("{}.field({})", path, java_string(key)),
                            )
                        ),
                    );
                    ctor_args.push(format!("__f_{}.optUnwrap()", ident));
                },
            }
        }
        let decode =
            format!(
                "{locals}for (var __fe : {jvs}.object({json}, {path}).entrySet()) {{\n    String __fname = __fe.getKey();\n    switch (__fname) {{\n{cases}        default -> {{\n            {unknown}\n        }}\n    }}\n}}\n{checks}",
                locals = dec_locals,
                jvs = self.rt("JsonValues"),
                json = json,
                path = path,
                cases = indent(&dec_cases, 2),
                unknown =
                    self.exception(
                        &format!("\"Unknown field '\" + __fname + \"' in {}\"", name),
                        &format!("{}.field(__fname)", path),
                    ),
                checks = dec_checks,
            );
        return RecordParts {
            components: components.join(", "),
            encode: format!(
                "{om}<String, {jv}> __o = new {om}<>();\n{enc}",
                om = self.pre("TOrderedMap"),
                jv = self.rt("JsonValue"),
                enc = enc,
            ),
            decode: decode,
            ctor: ctor_args.join(", "),
        };
    }

    fn root_methods(&self, name: &str, ty: &str) -> String {
        if self.schema.default.as_deref() != Some(name) {
            return String::new();
        }
        return format!(
            "\n    public static String serialize({n} value) {{\n        return {jvs}.serialize(toValue(value));\n    }}\n\n    public static {n} deserialize(String json) throws java.io.IOException {{\n        return fromValue({jvs}.parse(json), {j}.FieldPath.root());\n    }}\n",
            n = ty,
            jvs = self.rt("JsonValues"),
            j = self.rt("Json"),
        );
    }

    fn encode(&mut self, maskoid: &Maskoid, hint: &str, expr: &str, out: &str) -> String {
        let jv = self.rt("JsonValue");
        return match maskoid {
            Maskoid::Null => format!("{} {} = new {}();\n", jv, out, self.rt("JsonValueNull")),
            Maskoid::String | Maskoid::ConstString(_) => format!(
                "{} {} = new {}({});\n",
                jv,
                out,
                self.rt("JsonValueString"),
                expr
            ),
            Maskoid::Bool => format!("{} {} = new {}({});\n", jv, out, self.rt("JsonValueBoolean"), expr),
            Maskoid::Int => format!("{} {} = new {}({});\n", jv, out, self.rt("JsonValueNumberInt"), expr),
            Maskoid::Float => format!("{} {} = new {}({});\n", jv, out, self.rt("JsonValueNumberFloat"), expr),
            Maskoid::Any => format!("{} {} = {};\n", jv, out, expr),
            Maskoid::Ref(name) => format!("{} {} = {}Codec.toValue({});\n", jv, out, name, expr),
            Maskoid::Option(inner) => {
                let unwrapped = format!("{}.optUnwrap()", expr);
                let some = self.tmp("j");
                let body = self.encode_option_inner(inner, hint, &unwrapped, &some);
                format!(
                    "{jv} {out};\nif ({expr}.optIsSome()) {{\n{body}    {out} = {some};\n}} else {{\n    {out} = new {null}();\n}}\n",
                    jv = jv,
                    out = out,
                    expr = expr,
                    body = indent(&body, 1),
                    some = some,
                    null = self.rt("JsonValueNull"),
                )
            },
            Maskoid::Set(inner) | Maskoid::List(inner) => {
                let list = self.tmp("l");
                let element = self.tmp("e");
                let element_out = self.tmp("j");
                let body = self.encode(inner, hint, &element, &element_out);
                format!(
                    "{tl}<{jv}> {list} = new {tl}<>();\nfor (var {element} : {expr}) {{\n{body}    {list}.add({element_out});\n}}\n{jv} {out} = new {ja}({list});\n",
                    tl = self.pre("TList"),
                    jv = jv,
                    list = list,
                    element = element,
                    expr = expr,
                    body = indent(&body, 1),
                    element_out = element_out,
                    out = out,
                    ja = self.rt("JsonValueArray"),
                )
            },
            Maskoid::StringMap(inner) => {
                let map = self.tmp("m");
                let entry = self.tmp("e");
                let element_out = self.tmp("j");
                let body = self.encode(inner, hint, &format!("{}.getValue()", entry), &element_out);
                format!(
                    "{om}<String, {jv}> {map} = new {om}<>();\nfor (var {entry} : {expr}.entrySet()) {{\n{body}    {map}.put({entry}.getKey(), {element_out});\n}}\n{jv} {out} = new {jo}({map});\n",
                    om = self.pre("TOrderedMap"),
                    jv = jv,
                    map = map,
                    entry = entry,
                    expr = expr,
                    body = indent(&body, 1),
                    element_out = element_out,
                    out = out,
                    jo = self.rt("JsonValueObject"),
                )
            },
            Maskoid::Tuple(_) | Maskoid::Record(_) | Maskoid::TaggedUnion(_) => {
                self.ensure_named(maskoid, hint);
                format!("{} {} = {}Codec.toValue({});\n", jv, out, hint, expr)
            },
        };
    }

    fn encode_option_inner(&mut self, inner: &Maskoid, hint: &str, expr: &str, out: &str) -> String {
        if matches!(inner, Maskoid::Option(_)) {
            let wrapper = self.option_inner_type(inner, hint);
            return format!("{} {} = {}Codec.toValue({});\n", self.rt("JsonValue"), out, wrapper, expr);
        }
        return self.encode(inner, hint, expr, out);
    }

    fn decode(&mut self, maskoid: &Maskoid, hint: &str, json: &str, path: &str, out: &str) -> String {
        let ty = self.type_of(maskoid, hint, false);
        return match maskoid {
            Maskoid::Null => format!(
                "if (!({json} instanceof {null})) {{\n    {exc}\n}}\n{ty} {out} = new {null}();\n",
                json = json,
                null = self.rt("JsonValueNull"),
                exc = self.exception("\"Expected null\"", path),
                ty = ty,
                out = out,
            ),
            Maskoid::String => format!("{} {} = {}.string({}, {});\n", ty, out, self.rt("JsonValues"), json, path),
            Maskoid::ConstString(expected) => format!(
                "{ty} {out} = {jvs}.string({json}, {path});\nif (!{out}.equals({expected})) {{\n    {exc}\n}}\n",
                ty = ty,
                out = out,
                jvs = self.rt("JsonValues"),
                json = json,
                path = path,
                expected = java_string(expected),
                exc = self.exception(&format!("\"Expected the string \" + {}", java_string(expected)), path),
            ),
            Maskoid::Bool => format!("{} {} = {}.bool({}, {});\n", ty, out, self.rt("JsonValues"), json, path),
            Maskoid::Int => format!("{} {} = {}.numberInt({}, {});\n", ty, out, self.rt("JsonValues"), json, path),
            Maskoid::Float => format!(
                "{ty} {out} = switch ({json}) {{\n    case {int} __n -> (double) __n.value();\n    case {float} __n -> __n.value();\n    default -> {exc}\n}};\n",
                ty = ty,
                out = out,
                json = json,
                int = self.rt("JsonValueNumberInt"),
                float = self.rt("JsonValueNumberFloat"),
                exc = self.exception("\"Expected number\"", path),
            ),
            Maskoid::Any => format!("{} {} = {};\n", ty, out, json),
            Maskoid::Ref(name) => format!("{} {} = {}Codec.fromValue({}, {});\n", ty, out, name, json, path),
            Maskoid::Option(inner) => {
                let v = self.tmp("v");
                let body = if matches!(inner.as_ref(), Maskoid::Option(_)) {
                    let wrapper = self.option_inner_type(inner, hint);
                    format!("{} {} = {}Codec.fromValue({}, {});\n", wrapper, v, wrapper, json, path)
                } else {
                    self.decode(inner, hint, json, path, &v)
                };
                format!(
                    "{ty} {out};\nif ({json} instanceof {null}) {{\n    {out} = {opt}.none();\n}} else {{\n{body}    {out} = {opt}.some({v});\n}}\n",
                    ty = ty,
                    out = out,
                    json = json,
                    null = self.rt("JsonValueNull"),
                    opt = self.pre("Opt"),
                    body = indent(&body, 1),
                    v = v,
                )
            },
            Maskoid::Set(inner) | Maskoid::List(inner) => {
                let index = self.tmp("i");
                let element = self.tmp("e");
                let v = self.tmp("v");
                let body = self.decode(inner, hint, &element, &format!("{}.index({})", path, index), &v);
                let (seen_decl, seen_check) = match maskoid {
                    Maskoid::Set(_) => {
                        let seen = self.tmp("s");
                        let inner_ty = self.type_of(inner, hint, true);
                        (
                            format!(
                                "{ts}<{t}> {seen} = new {ts}<>();\n",
                                ts = self.pre("TSet"),
                                t = inner_ty,
                                seen = seen
                            ),
                            format!(
                                "    if (!{seen}.add({v})) {{\n        {exc}\n    }}\n",
                                seen = seen,
                                v = v,
                                exc =
                                    self.exception(
                                        "\"Duplicate element in set\"",
                                        &format!("{}.index({})", path, index)
                                    ),
                            ),
                        )
                    },
                    _ => (String::new(), String::new()),
                };
                format!(
                    "{ty} {out} = new {tl}<>();\n{seen_decl}int {index} = 0;\nfor (var {element} : {jvs}.array({json}, {path})) {{\n{body}{seen_check}    {out}.add({v});\n    {index}++;\n}}\n",
                    ty = ty,
                    out = out,
                    tl = self.pre("TList"),
                    seen_decl = seen_decl,
                    index = index,
                    element = element,
                    jvs = self.rt("JsonValues"),
                    json = json,
                    path = path,
                    body = indent(&body, 1),
                    seen_check = seen_check,
                    v = v,
                )
            },
            Maskoid::StringMap(inner) => {
                let entry = self.tmp("e");
                let v = self.tmp("v");
                let body =
                    self.decode(
                        inner,
                        hint,
                        &format!("{}.getValue()", entry),
                        &format!("{}.field({}.getKey())", path, entry),
                        &v,
                    );
                format!(
                    "{ty} {out} = new {om}<>();\nfor (var {entry} : {jvs}.object({json}, {path}).entrySet()) {{\n{body}    {out}.put({entry}.getKey(), {v});\n}}\n",
                    ty = ty,
                    out = out,
                    om = self.pre("TOrderedMap"),
                    entry = entry,
                    jvs = self.rt("JsonValues"),
                    json = json,
                    path = path,
                    body = indent(&body, 1),
                    v = v,
                )
            },
            Maskoid::Tuple(_) | Maskoid::Record(_) | Maskoid::TaggedUnion(_) => {
                self.ensure_named(maskoid, hint);
                format!("{} {} = {}Codec.fromValue({}, {});\n", ty, out, hint, json, path)
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

pub fn generate_java(schema: &SchemaskV1, config: &JavaConfig) -> Vec<JavaFile> {
    let mut codegen = CodeGen {
        schema: schema,
        config: config,
        files: vec![],
        generated: BTreeSet::new(),
        counter: 0,
    };
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
                    "{header}{doc}public final class {n}Codec {{\n    private {n}Codec() {{}}\n\n    public static {jv} toValue({ty} value) {{\n{enc}        return {out};\n    }}\n\n    public static {ty} fromValue({jv} value, {json}.FieldPath path) {{\n{dec}        return {v};\n    }}\n{root}}}\n",
                    header = codegen.header(),
                    doc = javadoc(maskoid.description()),
                    n = name,
                    jv = codegen.rt("JsonValue"),
                    ty = ty,
                    enc = indent(&enc, 2),
                    out = out,
                    json = codegen.rt("Json"),
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
    return codegen.files;
}
