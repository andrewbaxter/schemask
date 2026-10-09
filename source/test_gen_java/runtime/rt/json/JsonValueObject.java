package rt.json;

public record JsonValueObject(rt.util.TOrderedMap<String, JsonValue> entries) implements JsonValue {}
