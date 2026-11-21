import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import gen.AccountCodec;
import gen.CoordsCodec;
import gen.EventCodec;
import gen.FooBarCodec;
import gen.FooCodec;
import gen.LabelCodec;
import gen.MaybeCodec;
import gen.MetaCodec;
import gen.ShapeCodec;
import gen.TagsCodec;
import gen.UniqueCodec;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * Test driver: `Main BINDING FILE` decodes the json in FILE as BINDING, re-encodes it and prints
 * the result on one line. Any failure exits non-zero.
 */
public final class Main {
    private Main() {}

    private static final ObjectMapper MAPPER =
            new ObjectMapper().enable(DeserializationFeature.FAIL_ON_TRAILING_TOKENS);

    public static void main(String[] args) throws Exception {
        var text = Files.readString(Path.of(args[1]), StandardCharsets.UTF_8);
        if (args[0].equals("Event")) {
            // The default binding gets string round trip helpers; exercise those.
            System.out.println(EventCodec.serialize(EventCodec.deserialize(text)));
            return;
        }
        JsonNode value = MAPPER.readTree(text);
        JsonNode out =
                switch (args[0]) {
                    case "Account" -> AccountCodec.toValue(AccountCodec.fromValue(value));
                    case "Coords" -> CoordsCodec.toValue(CoordsCodec.fromValue(value));
                    case "Foo" -> FooCodec.toValue(FooCodec.fromValue(value));
                    case "FooBar" -> FooBarCodec.toValue(FooBarCodec.fromValue(value));
                    case "Label" -> LabelCodec.toValue(LabelCodec.fromValue(value));
                    case "Maybe" -> MaybeCodec.toValue(MaybeCodec.fromValue(value));
                    case "Meta" -> MetaCodec.toValue(MetaCodec.fromValue(value));
                    case "Shape" -> ShapeCodec.toValue(ShapeCodec.fromValue(value));
                    case "Tags" -> TagsCodec.toValue(TagsCodec.fromValue(value));
                    case "Unique" -> UniqueCodec.toValue(UniqueCodec.fromValue(value));
                    default -> throw new IllegalArgumentException("Unknown binding " + args[0]);
                };
        System.out.println(MAPPER.writeValueAsString(out));
    }
}
