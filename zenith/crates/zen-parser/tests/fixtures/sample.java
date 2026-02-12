package zenith.sample;

import java.util.List;
import static java.util.Collections.emptyList;

public interface Renderer {
    int VERSION = 1;
    String render(String input);
}

public enum Status {
    NEW,
    READY,
    DONE
}

public record Point(int x, int y) {}

public @interface Label {
    String value();
}

public class Widget implements Renderer {
    public static final int MAX_SIZE = 128;
    private int count;
    final int seed;

    public Widget(int count) {
        this.count = count;
        this.seed = count;
    }

    /** Renders a widget. */
    @Override
    public String render(String input) {
        return input + count;
    }

    public List<String> items() {
        return emptyList();
    }

    String packageName() {
        return "widget";
    }
}
