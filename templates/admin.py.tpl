from gingerdj.contrib import admin

from .models import *

def create_model_admin(model):
    class ModelAdmin(admin.ModelAdmin):
        list_display = [field.name for field in model._meta.fields]
        search_fields = [field.name for field in model._meta.fields if isinstance(
            field, models.CharField)]
        list_filter = [field.name for field in model._meta.fields]

    return ModelAdmin


{% for schema in schemas %}
{% if schema.type == 'table' %}
admin.site.register({{schema.data.table_name}}, create_model_admin({{schema.data.table_name}}) )
{% endif %}{% endfor %}
