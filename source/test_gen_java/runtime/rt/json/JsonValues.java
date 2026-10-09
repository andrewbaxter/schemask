package rt.json;

import java.io.IOException;
import rt.util.TList;
import rt.util.TOrderedMap;

/** Stand-in for the json runtime: a small parser, a serializer and the typed accessors. */
public final class JsonValues {
    private JsonValues() {}

    public static long numberInt(JsonValue value, Json.FieldPath path) {
        return switch (value) {
            case JsonValueNumberInt n -> n.value();
            default -> throw new Json.DeserializationException("Expected integer", path);
        };
    }

    public static String string(JsonValue value, Json.FieldPath path) {
        return switch (value) {
            case JsonValueString s -> s.value();
            default -> throw new Json.DeserializationException("Expected string", path);
        };
    }

    public static boolean bool(JsonValue value, Json.FieldPath path) {
        return switch (value) {
            case JsonValueBoolean b -> b.value();
            default -> throw new Json.DeserializationException("Expected boolean", path);
        };
    }

    public static TList<JsonValue> array(JsonValue value, Json.FieldPath path) {
        return switch (value) {
            case JsonValueArray a -> a.elements();
            default -> throw new Json.DeserializationException("Expected array", path);
        };
    }

    public static TOrderedMap<String, JsonValue> object(JsonValue value, Json.FieldPath path) {
        return switch (value) {
            case JsonValueObject o -> o.entries();
            default -> throw new Json.DeserializationException("Expected object", path);
        };
    }

    public static String serialize(JsonValue value) {
        var out = new StringBuilder();
        write(out, value);
        return out.toString();
    }

    private static void write(StringBuilder out, JsonValue value) {
        switch (value) {
            case JsonValueNull ignored -> out.append("null");
            case JsonValueBoolean b -> out.append(b.value());
            case JsonValueNumberInt n -> out.append(n.value());
            case JsonValueNumberFloat n -> out.append(n.value());
            case JsonValueString s -> writeString(out, s.value());
            case JsonValueArray a -> {
                out.append('[');
                var first = true;
                for (var e : a.elements()) {
                    if (!first) out.append(',');
                    first = false;
                    write(out, e);
                }
                out.append(']');
            }
            case JsonValueObject o -> {
                out.append('{');
                var first = true;
                for (var e : o.entries().entrySet()) {
                    if (!first) out.append(',');
                    first = false;
                    writeString(out, e.getKey());
                    out.append(':');
                    write(out, e.getValue());
                }
                out.append('}');
            }
        }
    }

    private static void writeString(StringBuilder out, String s) {
        out.append('"');
        for (var c : s.toCharArray()) {
            switch (c) {
                case '"' -> out.append("\\\"");
                case '\\' -> out.append("\\\\");
                case '\n' -> out.append("\\n");
                default -> out.append(c);
            }
        }
        out.append('"');
    }

    public static JsonValue parse(String text) throws IOException {
        var parser = new Parser(text);
        var value = parser.value();
        parser.skipWhitespace();
        if (parser.at < text.length()) {
            throw new IOException("Trailing content at " + parser.at);
        }
        return value;
    }

    private static final class Parser {
        private final String text;
        private int at = 0;

        Parser(String text) {
            this.text = text;
        }

        void skipWhitespace() {
            while (at < text.length() && Character.isWhitespace(text.charAt(at))) at++;
        }

        char peek() throws IOException {
            if (at >= text.length()) throw new IOException("Unexpected end");
            return text.charAt(at);
        }

        void expect(String word) throws IOException {
            if (!text.startsWith(word, at)) throw new IOException("Expected " + word + " at " + at);
            at += word.length();
        }

        JsonValue value() throws IOException {
            skipWhitespace();
            var c = peek();
            if (c == '{') {
                at++;
                var entries = new TOrderedMap<String, JsonValue>();
                skipWhitespace();
                if (peek() == '}') {
                    at++;
                    return new JsonValueObject(entries);
                }
                while (true) {
                    skipWhitespace();
                    var key = string();
                    skipWhitespace();
                    expect(":");
                    entries.put(key, value());
                    skipWhitespace();
                    if (peek() == ',') {
                        at++;
                        continue;
                    }
                    expect("}");
                    return new JsonValueObject(entries);
                }
            }
            if (c == '[') {
                at++;
                var elements = new TList<JsonValue>();
                skipWhitespace();
                if (peek() == ']') {
                    at++;
                    return new JsonValueArray(elements);
                }
                while (true) {
                    elements.add(value());
                    skipWhitespace();
                    if (peek() == ',') {
                        at++;
                        continue;
                    }
                    expect("]");
                    return new JsonValueArray(elements);
                }
            }
            if (c == '"') return new JsonValueString(string());
            if (c == 't') {
                expect("true");
                return new JsonValueBoolean(true);
            }
            if (c == 'f') {
                expect("false");
                return new JsonValueBoolean(false);
            }
            if (c == 'n') {
                expect("null");
                return new JsonValueNull();
            }
            var start = at;
            while (at < text.length() && "+-0123456789.eE".indexOf(text.charAt(at)) >= 0) at++;
            var number = text.substring(start, at);
            if (number.isEmpty()) throw new IOException("Unexpected character at " + start);
            if (number.contains(".") || number.contains("e") || number.contains("E")) {
                return new JsonValueNumberFloat(Double.parseDouble(number));
            }
            return new JsonValueNumberInt(Long.parseLong(number));
        }

        String string() throws IOException {
            expect("\"");
            var out = new StringBuilder();
            while (true) {
                var c = peek();
                at++;
                if (c == '"') return out.toString();
                if (c == '\\') {
                    var e = peek();
                    at++;
                    switch (e) {
                        case 'n' -> out.append('\n');
                        case 't' -> out.append('\t');
                        case 'u' -> {
                            out.append((char) Integer.parseInt(text.substring(at, at + 4), 16));
                            at += 4;
                        }
                        default -> out.append(e);
                    }
                    continue;
                }
                out.append(c);
            }
        }
    }
}
