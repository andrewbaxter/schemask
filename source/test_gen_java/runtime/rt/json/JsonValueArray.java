package rt.json;

public record JsonValueArray(rt.util.TList<JsonValue> elements) implements JsonValue {}
