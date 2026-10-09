package rt.util;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

/** Stand-in for the prelude list type, with the surface the generated code uses. */
public final class TList<T> implements Iterable<T> {
    private final List<T> inner = new ArrayList<>();

    @SafeVarargs
    public static <T> TList<T> of(T... elements) {
        var list = new TList<T>();
        for (var e : elements) {
            list.add(e);
        }
        return list;
    }

    public int size() {
        return inner.size();
    }

    public T get(int index) {
        return inner.get(index);
    }

    public void add(T element) {
        inner.add(element);
    }

    @Override
    public Iterator<T> iterator() {
        return inner.iterator();
    }

    @Override
    public boolean equals(Object other) {
        return other instanceof TList<?> o && inner.equals(o.inner);
    }

    @Override
    public int hashCode() {
        return inner.hashCode();
    }
}
