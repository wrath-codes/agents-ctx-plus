open module zenith.sample.module {
    exports zenith.sample;
    opens zenith.sample.internal;
    requires java.base;
    requires static java.sql;
    uses zenith.sample.spi.Renderer;
    provides zenith.sample.spi.Renderer with zenith.sample.impl.DefaultRenderer;
}
