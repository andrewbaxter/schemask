package rt.json;

public sealed interface JsonValue permits JsonValueNull, JsonValueBoolean, JsonValueNumberInt, JsonValueNumberFloat, JsonValueString, JsonValueArray, JsonValueObject {}
