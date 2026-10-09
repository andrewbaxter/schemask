package rt.util;

import java.util.HashSet;
import java.util.Set;

/** Stand-in for the prelude set type, with the surface the generated code uses. */
public final class TSet<T> {
    private final Set<T> inner = new HashSet<>();

    public boolean add(T element) {
        return inner.add(element);
    }
}
