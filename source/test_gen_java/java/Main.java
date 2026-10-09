import gen.AccountCodec;
import gen.CoordsCodec;
import gen.EventCodec;
import gen.LabelCodec;
import gen.MaybeCodec;
import gen.MetaCodec;
import gen.ShapeCodec;
import gen.TagsCodec;
import gen.UniqueCodec;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import rt.json.Json;
import rt.json.JsonValues;

/**
 * Test driver: `Main BINDING FILE` decodes the json in FILE as BINDING, re-encodes it and prints
 * the result on one line. Any failure exits non-zero.
 */
public final class Main {
    private Main() {}

    public static void main(String[] args) throws Exception {
        var text = Files.readString(Path.of(args[1]), StandardCharsets.UTF_8);
        var root = Json.FieldPath.root();
        var value = JsonValues.parse(text);
        var out =
                switch (args[0]) {
                    case "Event" -> EventCodec.serialize(EventCodec.deserialize(text));
                    case "Account" -> JsonValues.serialize(AccountCodec.toValue(AccountCodec.fromValue(value, root)));
                    case "Coords" -> JsonValues.serialize(CoordsCodec.toValue(CoordsCodec.fromValue(value, root)));
                    case "Label" -> JsonValues.serialize(LabelCodec.toValue(LabelCodec.fromValue(value, root)));
                    case "Maybe" -> JsonValues.serialize(MaybeCodec.toValue(MaybeCodec.fromValue(value, root)));
                    case "Meta" -> JsonValues.serialize(MetaCodec.toValue(MetaCodec.fromValue(value, root)));
                    case "Shape" -> JsonValues.serialize(ShapeCodec.toValue(ShapeCodec.fromValue(value, root)));
                    case "Tags" -> JsonValues.serialize(TagsCodec.toValue(TagsCodec.fromValue(value, root)));
                    case "Unique" -> JsonValues.serialize(UniqueCodec.toValue(UniqueCodec.fromValue(value, root)));
                    default -> throw new IllegalArgumentException("Unknown binding " + args[0]);
                };
        System.out.println(out);
    }
}
