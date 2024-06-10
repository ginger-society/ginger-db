from ginger.db import models

{% for schema in schemas %}
{% if schema.type == 'table' %}
class {{ schema.data.table_name }}(models.Model):
    {% for row in schema.rows %}
    {{ row.data.field_name }} = models.{{ row.data.type }}({% if row.data.null %}null=True, {% endif %}{% if row.data.on_delete %}on_delete=models.{{ row.data.on_delete }}{% endif %}) {% endfor %}
{% elif schema.type == 'enum' %}
{{schema.id}} = (
    {% for opt in schema.data.options  %}
        ("{{opt.value}}", "{{opt.label}}"),
    {% endfor %}
)
{%  endif %}

{% endfor %}
