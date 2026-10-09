package rt.util;

import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Set;

/** Stand-in for the prelude ordered map type, with the surface the generated code uses. */
public final class TOrderedMap<K, V> {
    private final Map<K, V> inner = new LinkedHashMap<>();

    public int size() {
        return inner.size();
    }

    public boolean containsKey(K key) {
        return inner.containsKey(key);
    }

    public void put(K key, V value) {
        inner.put(key, value);
    }

    public Set<Map.Entry<K, V>> entrySet() {
        return inner.entrySet();
    }

    @Override
    public boolean equals(Object other) {
        return other instanceof TOrderedMap<?, ?> o && inner.equals(o.inner);
    }

    @Override
    public int hashCode() {
        return inner.hashCode();
    }
}
