from ginger.contrib import admin

from .models import *


def create_model_admin(model):
    class ModelAdmin(admin.ModelAdmin):
        list_display = [field.name for field in model._meta.fields]
        search_fields = [field.name for field in model._meta.fields if isinstance(
            field, models.CharField)]
        list_filter = [field.name for field in model._meta.fields]

    return ModelAdmin


admin.site.register(student, create_model_admin(student))


admin.site.register(enrollment, create_model_admin(enrollment))


admin.site.register(course, create_model_admin(course))


admin.site.register(exam, create_model_admin(exam))
