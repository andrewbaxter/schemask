package rt.util;

/** Stand-in for the prelude option type, with the surface the generated code uses. */
public sealed interface Opt<T> {
    @SuppressWarnings("unchecked")
    static <T> Opt<T> none() {
        return (Opt<T>) None.INSTANCE;
    }

    static <T> Opt<T> some(T value) {
        return new Some<>(value);
    }

    boolean optIsSome();

    boolean optIsNone();

    T optUnwrap();

    record Some<T>(T value) implements Opt<T> {
        @Override
        public boolean optIsSome() {
            return true;
        }

        @Override
        public boolean optIsNone() {
            return false;
        }

        @Override
        public T optUnwrap() {
            return value;
        }
    }

    record None<T>() implements Opt<T> {
        private static final None<?> INSTANCE = new None<>();

        @Override
        public boolean optIsSome() {
            return false;
        }

        @Override
        public boolean optIsNone() {
            return true;
        }

        @Override
        public T optUnwrap() {
            throw new IllegalStateException("none");
        }
    }
}
