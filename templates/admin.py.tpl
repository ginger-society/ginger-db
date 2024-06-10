from ginger.contrib import admin

from .models import *

{% for schema in schemas %}
{% if schema.type == 'table' %}
admin.site.register({{schema.data.table_name}})
{% endif %}{% endfor %}
