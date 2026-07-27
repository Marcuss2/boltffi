{% if let Some(doc) = constant.doc() %}{{ doc }}
{% endif %}{% if let Some(value) = constant.value() %}    public static final {{ constant.ty() }} {{ constant.name() }} = {{ value }};
{% endif %}{% if let Some(accessor) = constant.accessor() %}{% if let Some(holder) = constant.holder() %}    public static {{ constant.ty() }} {{ constant.name() }}() {
        return {{ holder }}.VALUE;
    }

    private static final class {{ holder }} {
        static final {{ constant.ty() }} VALUE = {{ accessor.name() }}();
    }

    private static {{ accessor.returns() }} {{ accessor.name() }}() {
{% for statement in accessor.body() %}        {{ statement }}
{% endfor %}    }
{% endif %}{% endif %}
