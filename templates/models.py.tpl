from gingerdj.db import models

{% for schema in schemas %}
{% if schema.type == 'table' %}
class {{ schema.data.table_name }}(models.Model):
        """{{schema.data.docs}}"""
    {% for row in schema.rows %}
    {% if row.id != 'pk' %} 
        {{ row.data.field_name }} = models.{{ row.data.type }}({% if row.data.target %}'{{row.data.target}}',{% endif %}{% if row.data.related_name %}related_name = '{{row.data.related_name}}', {% endif %}{% if row.data.on_delete %}on_delete=models.{{ row.data.on_delete }},{% endif %}{% if row.data.type == 'BooleanField' %}default={% if row.data.default %}{{'True'}}{% else %}{{'False'}}{% endif %},{% endif %} {% if row.data.options_target %}choices={{row.data.options_target}},{% endif %} {% if row.data.max_length %}max_length={{row.data.max_length}},{% endif %}  {% if row.data.type == 'CharField' or row.data.type == 'TextField' %} {% if row.data.default %}default='{{row.data.default}}',{% endif %}{% endif %}  {% if row.data.null %}{% if row.data.type == 'CharField' or row.data.type == 'ForeignKey' %}blank=True,null=True,{% else %}null=True,{% endif %} {% endif %}  {% if row.data.auto_now %}auto_now=True, {% endif %} {% if row.data.auto_now_add %}auto_now_add=True, {% endif %} )
    {% endif %}{% endfor %}
        class Meta:
            db_table = "{{schema.data.table_name}}"
{% elif schema.type == 'enum' %}
{{schema.id}} = (
    {% for opt in schema.data.options  %}
        ("{{opt.value}}", "{{opt.label}}"),
    {% endfor %}
)
{%  endif %}

{% endfor %}
