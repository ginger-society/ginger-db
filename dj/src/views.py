# pylint: disable=C0302
"""
    Defines shared views aka handlers
"""

import copy
import json
import secrets
import string
from datetime import datetime, timedelta
from os import listdir
from os.path import isfile, join

import ginger.apps
from ginger.db import models
from ginger.db.models.deletion import CASCADE, SET_NULL
from ginger.db.models.fields.related import ForeignKey, ManyToManyField
from ginger.http import JsonResponse
from ginger.template.loader import render_to_string
from ginger.drf_yasg import openapi
from ginger.drf_yasg.utils import swagger_auto_schema
from ginger.rest_framework import serializers
from ginger.rest_framework.decorators import api_view

# from rest_framework.fields import empty


ORM_TYPE_MAPPER = {
    "typeORM": {
        "CharField": "string",
        "BooleanField": "boolean",
        "BigAutoField": "number",
        "DateField": "string",
        "EmailField": "string",
        "DateTimeField": "Date",
        "PositiveIntegerField": "number",
        "ForeignKey": "number",
        "ManyToManyField": "number",
        "AutoField": "number",
        "TextField": "string",
        "PositiveSmallIntegerField": "number",
    },
    "py-sqlalchemy": {
        "CharField": "String",
        "BooleanField": "Boolean",
        "BigAutoField": "Integer",
        "DateField": "Date",
        "EmailField": "String",
        "DateTimeField": "DateTime",
        "PositiveIntegerField": "Integer",
        "ForeignKey": "ForeignKey",
        "ManyToManyField": "Integer",
        "AutoField": "Integer",
        "TextField": "String",
        "PositiveSmallIntegerField": "Integer",
    },
    "rust-diesel": {
        "CharField": "Varchar",
        "BooleanField": "Bool",
        "BigAutoField": "BigInt",
        "DateField": "Date",
        "EmailField": "Varchar",
        "DateTimeField": "Timestamptz",
        "PositiveIntegerField": "Integer",
        "ForeignKey": "BigInt",
        "ManyToManyField": "BigInt",
        "AutoField": "BigInt",
        "TextField": "Varchar",
        "PositiveSmallIntegerField": "BigInt",
    },
}

LANG_TYPE_MAPPER = {
    "py-sqlalchemy": {
        "CharField": "str",
        "BooleanField": "bool",
        "BigAutoField": "int",
        "DateField": "Date",
        "EmailField": "str",
        "DateTimeField": "str",
        "PositiveIntegerField": "int",
        "ForeignKey": "int",
        "ManyToManyField": "int",
        "AutoField": "int",
        "TextField": "str",
        "PositiveSmallIntegerField": "int",
    },
    "rust-diesel": {
        "CharField": "String",
        "BooleanField": "bool",
        "BigAutoField": "i64",
        "DateField": "NaiveDate",
        "EmailField": "String",
        "DateTimeField": "DateTime<Utc>",
        "PositiveIntegerField": "i32",
        "ForeignKey": "i64",
        "ManyToManyField": "i64",
        "AutoField": "i64",
        "TextField": "String",
        "PositiveSmallIntegerField": "i64",
    },
}


def get_model_db_schemas(models_to_render, orm):  # noqa: C901 # pylint: disable=R0915,R0912,R0914
    """
    It gets DB schema defined in all apps
    """

    schema_obj = {}
    models_list = []

    for model in ginger.apps.apps.get_models():
        if model.__name__ not in models_to_render:
            # print("continuing for ", model.__module__)
            continue
        models_list.append(model)

    model_index = 0
    # print(models_list)
    while model_index < len(models_list):  # pylint: disable= R1702
        # PROCESSING MODEL
        # print("\n\n\n")

        model = models_list[model_index]
        model_index += 1
        # print(model in models_list)
        if model._meta.proxy:
            continue

            # continue
        # print(model.__dict__)
        field_items = model.__dict__.items()
        if hasattr(model, "Meta"):  # and hasattr(model.Meta, "proxy"):
            # print(model.Meta.__dict__)
            # if hasattr(model.__dict__ , '__dict__'):
            #     print('*****', model.__dict__)
            # print('++++', dir(model.Meta))
            parent_model = model.__mro__[1]
            if parent_model.__module__.split(".")[1] == "models":
                field_items = parent_model.__dict__.items()
            # print('------' + model.__name__ , parent_model)
        # if model.__name__ != 'Tenant':
        #     continue
        schema_obj[model.__name__] = {}
        if model.__doc__:
            schema_obj[model.__name__]["__doc__"] = model.__doc__

        schema_obj[model.__name__]["fields"] = {}
        schema_obj[model.__name__]["table_name"] = model.objects.model._meta.db_table

        schema_obj[model.__name__]["exports"] = []
        schema_obj[model.__name__]["model_imports"] = []
        schema_obj[model.__name__]["relation_imports"] = []
        schema_obj[model.__name__]["relation_tables"] = []
        schema_obj[model.__name__]["rust_derive"] = []
        schema_obj[model.__name__]["rust_decorations"] = []
        schema_obj[model.__name__]["rust_related_models"] = []

        for field, field_def in field_items:
            # FIELD PROCESSING

            anchor_model = None

            if isinstance(field_def, property):
                continue

            if (
                not isinstance(field_def, str)  # pylint: disable=R0916
                and not isinstance(field_def, list)
                and (
                    field_def.__module__.startswith("ginger.db.models.query")
                    or (hasattr(field_def, "field") and isinstance(field_def.field, ForeignKey) and not field.endswith("_id"))
                    or (hasattr(field_def, "field") and isinstance(field_def.field, ManyToManyField))
                )
            ):
                on_delete = None
                relation_decorator = None
                if isinstance(field_def.field, ForeignKey):
                    if "rel" in field_def.__dict__:
                        # print("o2m field name", field_def.__dict__["field"])
                        # print("o2m target field name", field_def.__dict__["field"].column)
                        # print(dir(field_def.__dict__["field"]))
                        target_model_name = str(
                            field_def.__dict__["field"]).split(".")[1]
                        relation_decorator = "OneToMany"
                        target_column_name = field_def.__dict__["field"].column
                        target_app = str(field_def.__dict__["field"]).split(
                            ".", maxsplit=1)[0]
                        # print("field_def.field", field_def.field.__module__)
                        # if field_def.field not in models_list:
                        #     models_list.append(field_def.field)
                    else:
                        # print("m2o target name", field_def.field.target_field)
                        # print("m2o target field name", field_def.field.target_field.column)
                        # print("m2o target name" , field_def.field.target_field.__module__)
                        # print(dir(field_def.field.target_field))
                        target_model_name = str(
                            field_def.field.target_field).split(".")[1]
                        target_column_name = field_def.field.target_field.column
                        target_app = str(field_def.field.target_field).split(
                            ".", maxsplit=1)[0]
                        relation_decorator = "ManyToOne"
                        target_table_name = field_def.field.target_field.model._meta.db_table

                        # print(dir(field_def.field), field_def.field.remote_field.on_delete)
                        # print("field_def.field.target_field", field_def.field.related_model)

                        on_delete = field_def.field.remote_field.on_delete

                        if field_def.field.related_model not in models_list:
                            models_list.append(field_def.field.related_model)

                    # if str(field_def.field).startswith('admin.'):
                    #     print("skipping for admin. fields foreign key id", field, field_def.field.target_field)
                    #     continue
                # if isinstance(field_def.field, ManyToManyField):
                #     print("many to many field found---------")
                #     print(field_def.field)
                #     print(dir(field_def.field))
                #     print(field_def.field.db_tablespace)
                #     print(field_def.field.auto_created)

                # print(field_def.field.__class__.__name__)
                # print('is null ' , field_def.field.null)
                # print('default value ' , field_def.field.default)
                # print('choices ' , field_def.field.choices)
                # print(dir(field_def.field), field_def.field.db_column)
                # print("---------\n\n")

                schema_obj[model.__name__]["fields"][field] = {
                    "type": ORM_TYPE_MAPPER[orm][field_def.field.__class__.__name__],
                    "pyType": field_def.field.__class__.__name__,
                    "null": field_def.field.null,
                    "decorators": [],
                    "no_type": False,
                    "relation_decorator": relation_decorator,
                }

                if orm == "py-sqlalchemy":
                    schema_obj[model.__name__]["fields"][field]["pydenticType"] = LANG_TYPE_MAPPER[orm][field_def.field.__class__.__name__]
                    schema_obj[model.__name__]["fields"][field]["typeInputs"] = []
                    schema_obj[model.__name__]["fields"][field]["decoratorType"] = "mapped_column"
                if orm == "rust-diesel":
                    schema_obj[model.__name__]["fields"][field]["rustType"] = LANG_TYPE_MAPPER[orm][field_def.field.__class__.__name__]

                if field_def.field.default.__class__ != type and not callable(field_def.field.default):
                    schema_obj[model.__name__]["fields"][field]["default"] = field_def.field.default
                if field_def.field.choices:
                    if isinstance(field_def.field.choices[0][0], int):
                        schema_obj[model.__name__]["fields"][field]["trsnaform_enum"] = True
                    else:
                        schema_obj[model.__name__]["fields"][field]["trsnaform_enum"] = False
                    schema_obj[model.__name__]["fields"][field]["choices"] = field_def.field.choices

                    if orm == "typeORM":
                        schema_obj[model.__name__]["exports"].append(
                            field + "Enum")

                        choice_field_decorator_input = {
                            "type": '"enum"',
                            "enum": field + "Enum",
                        }
                        if not isinstance(field_def.field.default, type):
                            choice_field_decorator_input["default"] = field + \
                                "Enum." + field_def.field.default
                        schema_obj[model.__name__]["fields"][field]["decorators"].append(
                            {
                                "tag": "Column",
                                "input": choice_field_decorator_input,
                            }
                        )

                        schema_obj[model.__name__]["fields"][field]["type"] = field + "Enum"
                    elif orm == "py-sqlalchemy":
                        schema_obj[model.__name__]["fields"][field][
                            "type"] = "Enum(" + field + "Enum)"

                elif field_def.field.primary_key:
                    if orm == "typeORM":
                        schema_obj[model.__name__]["fields"][field]["decorators"].append(
                            {"tag": "PrimaryGeneratedColumn"})
                    elif orm == "py-sqlalchemy":
                        schema_obj[model.__name__]["fields"][field]["decorators"].append(
                            "primary_key=True")
                    elif orm == "rust-diesel":
                        schema_obj[model.__name__]["primaryKey"] = field
                    schema_obj[model.__name__]["fields"][field]["primary_key"] = True

                elif isinstance(  # pylint: disable=W1116
                    field_def.field,
                    models.CharField | models.TextField,
                ):
                    max_length = field_def.field.max_length
                    schema_obj[model.__name__]["fields"][field]["max_length"] = max_length
                    if orm == "py-sqlalchemy" and max_length:
                        schema_obj[model.__name__]["fields"][field]["typeInputs"].append(
                            max_length)
                elif isinstance(field_def.field, ManyToManyField):
                    relation_decorator = "ManyToMany"
                    if hasattr(field_def, "rel") and field_def.rel.model is model:
                        # print("if", field, field_def.rel.model, model, field_def.rel.model is model)

                        target_model_name = str(field_def.rel.related_model.__name__).split(
                            ".")[-1]  # pylint: disable=C0207
                        anchor_model = False

                        related_name = str(field_def.rel.field).split(
                            ".")[-1]  # pylint: disable=C0207
                        # print(field_def.rel.related_model)
                        # print(field_def.rel.related_model.__module__)
                        # print(field_def.rel.related_model in models_list)
                        if field_def.rel.related_model not in models_list:
                            models_list.append(field_def.rel.related_model)
                    else:
                        # print("else", field)
                        # print(field_def.rel.__dict__)
                        target_model_name = str(field_def.rel.model.__name__).split(
                            ".")[-1]  # pylint: disable=C0207
                        anchor_model = True
                        related_name = field_def.field._related_name  # pylint: disable=W0212

                        target_column_name = "id"
                        # print('----- , ' , field_def.rel.model.__module__)
                        # target_app = str(field_def.rel.model.__module__).split(".")[0]
                        target_table_name = field_def.rel.model._meta.db_table
                        # print(field_def.rel.model)
                        # print(field_def.rel.model.__module__)
                        # print(field_def.rel.model in models_list)
                        if field_def.rel.model not in models_list:
                            models_list.append(field_def.rel.model)

                    if not related_name:
                        related_name = model.__name__.split(
                            ".")[-1].lower() + "_set"

                    # print('------' , target_model_name, anchor_model)
                    if orm == "typeORM":
                        schema_obj[model.__name__]["fields"][field]["type"] = target_model_name + "Entity"
                        schema_obj[model.__name__]["model_imports"].append(
                            target_model_name)
                        schema_obj[model.__name__]["fields"][field]["foreign_key_target"] = target_model_name

                        # TODO:// rename foreign_key_target to related_target_model # pylint: disable=W0511
                        if relation_decorator not in schema_obj[model.__name__]["relation_imports"]:
                            schema_obj[model.__name__]["relation_imports"].append(
                                relation_decorator)

                        schema_obj[model.__name__]["fields"][field]["decorators"].append(
                            {
                                "tag": relation_decorator,
                                "inputStr": "() => "
                                + target_model_name
                                + "Entity, (currentModel:"
                                + target_model_name
                                + "Entity"
                                + ") => currentModel."
                                + related_name,
                            }
                        )

                        if anchor_model:
                            schema_obj[model.__name__]["fields"][field]["decorators"].append(
                                {
                                    "tag": "JoinTable",
                                    "inputStr": json.dumps(
                                        {
                                            "name": model.objects.model._meta.db_table + "_" + field,
                                            "joinColumn": {"name": model.__name__.lower() + "_id"},
                                            "inverseJoinColumn": {"name": target_model_name.lower() + "_id"},
                                        }
                                    ),
                                }
                            )
                    elif orm == "py-sqlalchemy":
                        schema_obj[model.__name__]["fields"][field]["decoratorType"] = "relationship"
                        schema_obj[model.__name__]["fields"][field]["no_type"] = True

                        schema_obj[model.__name__]["fields"][field]["decorators"].append(
                            'back_populates="' + related_name + '"')
                        schema_obj[model.__name__]["fields"][field]["pydenticType"] = "list[" + \
                            '"' + target_model_name + '"' + "]"

                        if anchor_model:
                            # print("anchor fielddef", field_def.field, target_column_name, target_app)

                            schema_obj[model.__name__]["fields"][field]["decorators"].append(
                                "secondary=" + model.__name__ + "_" + target_model_name)
                            schema_obj[model.__name__]["relation_tables"].append(
                                {
                                    "name": model.__name__ + "_" + target_model_name,
                                    "self_id": model.__name__.lower() + "_id",
                                    "other_id": target_model_name.lower() + "_id",
                                    "table": model.objects.model._meta.db_table + "_" + field,
                                    "self_id_path": model.objects.model._meta.db_table + ".id",
                                    "other_id_path": target_table_name + ".id",
                                }
                            )
                        else:
                            schema_obj[model.__name__]["fields"][field]["decorators"].append(
                                "secondary=" + target_model_name + "_" + model.__name__)

                    elif orm == "rust-diesel" and anchor_model:
                        schema_obj[model.__name__ + "_" + target_model_name] = {
                            "__doc__": "\n    ",
                            "fields": {
                                "id": {
                                    "type": "Int8",
                                    "pyType": "BigAutoField",
                                    "null": False,
                                    "decorators": [],
                                    "no_type": False,
                                    "relation_decorator": None,
                                    "rustType": "i64",
                                    "primary_key": True,
                                },
                                target_model_name.lower()
                                + "_id": {
                                    "type": "Int8",
                                    "pyType": "BigAutoField",
                                    "null": False,
                                    "decorators": [],
                                    "no_type": False,
                                    "relation_decorator": None,
                                    "rustType": "i64",
                                    "primary_key": False,
                                },
                                model.__name__.lower()
                                + "_id": {
                                    "type": "Int8",
                                    "pyType": "BigAutoField",
                                    "null": False,
                                    "decorators": [],
                                    "no_type": False,
                                    "relation_decorator": None,
                                    "rustType": "i64",
                                    "primary_key": False,
                                },
                            },
                            "primaryKey": "id",
                            "table_name": model.objects.model._meta.db_table + "_" + field,
                            "exports": [],
                            "model_imports": [],
                            "relation_imports": [],
                            "relation_tables": [],
                            "rust_derive": ["Associations"],
                            "rust_decorations": [
                                "#[diesel(belongs_to(" + target_model_name +
                                ", foreign_key = " + target_model_name.lower() + "_id" + "))]",
                                "#[diesel(belongs_to(" + model.__name__ +
                                ", foreign_key = " + model.__name__.lower() + "_id" + "))]",
                            ],
                            "rust_related_models": [],
                        }

                elif isinstance(field_def.field, ForeignKey):
                    if hasattr(field_def, "rel"):
                        related_name = str(field_def.rel.field).split(
                            ".")[-1]  # pylint: disable=C0207
                        # print("have attr..." , field_def, field, model)
                    else:
                        # print(dir(field_def.field))
                        # print(field , "does not have attr..." , field_def, model, model.__dict__)
                        related_name = field_def.field._related_name  # pylint: disable=W0212

                    if not related_name:
                        related_name = model.__name__.split(
                            ".")[-1].lower() + "_set"

                    schema_obj[model.__name__]["fields"][field]["foreign_key_target"] = target_model_name
                    schema_obj[model.__name__]["fields"][field]["target_app"] = target_app
                    if orm == "typeORM":
                        schema_obj[model.__name__]["fields"][field]["type"] = target_model_name + "Entity"

                        schema_obj[model.__name__]["model_imports"].append(
                            target_model_name)

                        if relation_decorator not in schema_obj[model.__name__]["relation_imports"]:
                            schema_obj[model.__name__]["relation_imports"].append(
                                relation_decorator)

                        if relation_decorator == "ManyToOne":
                            on_delete_decorator = ""

                            if on_delete:
                                if on_delete == CASCADE:  # pylint: disable=W0143
                                    on_delete_decorator = ",{ onDelete: 'CASCADE' }"
                                elif on_delete == SET_NULL:  # pylint: disable=W0143
                                    on_delete_decorator = ",{ onDelete: 'SET NULL' }"

                            schema_obj[model.__name__]["fields"][field]["decorators"].append(
                                {
                                    "tag": relation_decorator,
                                    "inputStr": "() => "
                                    + target_model_name
                                    + "Entity, (currentModel:"
                                    + target_model_name
                                    + "Entity"
                                    + ") => currentModel."
                                    + related_name
                                    + on_delete_decorator,
                                }
                            )

                            schema_obj[model.__name__]["fields"][field]["decorators"].append(
                                {
                                    "tag": "JoinColumn",
                                    "input": {"name": "'" + field + "_id'"},
                                }
                            )

                        else:
                            # get the name of the field where this relationship id defined

                            schema_obj[model.__name__]["fields"][field]["decorators"].append(
                                {
                                    "tag": relation_decorator,
                                    "inputStr": "() => "
                                    + target_model_name
                                    + "Entity, ("
                                    + target_model_name.lower()
                                    + ") => "
                                    + target_model_name.lower()
                                    + "."
                                    + related_name,
                                }
                            )
                    elif orm == "py-sqlalchemy":
                        if relation_decorator == "OneToMany":
                            schema_obj[model.__name__]["fields"][field]["pydenticType"] = 'list["' + \
                                target_model_name + '"]'
                            schema_obj[model.__name__]["fields"][field]["no_type"] = True
                            schema_obj[model.__name__]["fields"][field]["decorators"].append(
                                'back_populates="' + related_name + '"')

                            schema_obj[model.__name__]["fields"][field]["decoratorType"] = "relationship"
                            # print("\n\n\n")
                            # print("changing mapped_column to relationsip for" , field)
                        else:
                            schema_obj[model.__name__]["fields"][field]["pydenticType"] = '"' + \
                                target_model_name + '"'

                            # print(''"' + target_app + '_' + target_model_name.lower() + '.' + target_column_name + '"'', '"' + target_app + '_' + target_model_name.lower() + '.' + target_column_name + '"')
                            # print("field_def.field.target_field", field_def.field.target_field.model._meta.db_table)
                            # print(dir(field_def.field.target_field.related_model))

                            schema_obj[model.__name__]["fields"][field]["typeInputs"].append(
                                '"' + target_table_name + "." + target_column_name + '"')

                            field_copy = copy.deepcopy(
                                schema_obj[model.__name__]["fields"][field])
                            field_copy["pydenticType"] = "int"
                            schema_obj[model.__name__]["fields"][field]["decoratorType"] = "relationship"

                            schema_obj[model.__name__]["fields"][field]["decoratorType"] = "relationship"
                            # print("\n\n\n")
                            # print("changing mapped_column to relationsip for -2" , field)

                            schema_obj[model.__name__]["fields"][field]["decorators"].append(
                                'back_populates="' + related_name + '"')
                            schema_obj[model.__name__]["fields"][field]["no_type"] = True
                            schema_obj[model.__name__]["fields"][field_def.field.column] = field_copy
                    elif orm == "rust-diesel" and relation_decorator != "OneToMany":
                        schema_obj[model.__name__]["fields"][field]["alias"] = True
                        schema_obj[model.__name__]["fields"][field]["original_column_name"] = field_def.field.column

                        relationshipStr = (
                            "diesel::joinable!("
                            + model.objects.model._meta.db_table
                            + " -> "
                            + target_table_name
                            + " ("
                            + field_def.field.column
                            + "));"
                        )
                        if (
                            relationshipStr not in schema_obj[model.__name__]["relation_tables"]
                            and target_model_name not in schema_obj[model.__name__]["rust_related_models"]
                        ):
                            schema_obj[model.__name__]["relation_tables"].append(
                                relationshipStr)

                            schema_obj[model.__name__]["rust_decorations"].append(
                                "#[diesel(belongs_to(" + target_model_name +
                                ", foreign_key = " + field_def.field.column + "))]"
                            )

                            schema_obj[model.__name__]["rust_related_models"].append(
                                target_model_name)

                        if "Associations" not in schema_obj[model.__name__]["rust_derive"]:
                            schema_obj[model.__name__]["rust_derive"].append(
                                "Associations")

                        # print(field_def.field.column)
                # POST FIELD PROCESSING
                if orm == "typeORM":
                    if len(schema_obj[model.__name__]["fields"][field]["decorators"]) == 0:
                        # print("No decorator present, adding default decorator" , field)
                        schema_obj[model.__name__]["fields"][field]["decorators"].append({
                                                                                         "tag": "Column"})
                    if (
                        "ManyToOne" in schema_obj[model.__name__]["relation_imports"]
                        and "JoinColumn" not in schema_obj[model.__name__]["relation_imports"]
                    ):
                        schema_obj[model.__name__]["relation_imports"].append(
                            "JoinColumn")
                        # print("adding joinColumn in relation imports")
                    if (
                        "ManyToMany" in schema_obj[model.__name__]["relation_imports"]
                        and "JoinTable" not in schema_obj[model.__name__]["relation_imports"]
                        and anchor_model
                    ):
                        schema_obj[model.__name__]["relation_imports"].append(
                            "JoinTable")
                        # print("adding JoinTable in relation imports")

            # else:
            #     print("skipping for ", field)

    # print('-----------')
    # print(schema , '-----------')
    return schema_obj


def get_model_schema(request):
    """JSON endpoint handler to get all models defined in the this project"""
    models_to_render = request.GET["models"].split(",")
    return JsonResponse(get_model_db_schemas(models_to_render, "typeORM"))


def get_sqlalchemy_model_schema(request):
    """JSON endpoint to get all models for sqlalchemy"""
    models_to_render = request.GET["models"].split(",")
    return JsonResponse(get_model_db_schemas(models_to_render, "py-sqlalchemy"))


def get_diesel_model_schema(request):
    """JSON endpoint to get all models for disel ORM"""
    models_to_render = request.GET["models"].split(",")
    return JsonResponse(get_model_db_schemas(models_to_render, "rust-diesel"))


class ModelsReponseSerializer(serializers.Serializer):
    """test model"""

    name = serializers.CharField()
    doc = serializers.CharField()
    app_name = serializers.CharField()


@swagger_auto_schema(method="GET", responses={200: openapi.Response("get_all_models", ModelsReponseSerializer(many=True))}, security=[{"Bearer": []}])
@api_view(["GET"])
def get_all_defined_models(request):
    """Gets all defined tables in the DB"""
    to_return = []
    for model in ginger.apps.apps.get_models():
        app_name = model.__module__.replace(".models", "")

        to_return.append(
            {"name": model.__name__, "doc": model.__doc__, "app_name": app_name})
    sorted_data = sorted(to_return, key=lambda x: x["app_name"])
    return JsonResponse(sorted_data, safe=False)


lang_parameter = openapi.Parameter(
    "language", openapi.IN_QUERY, description="Language", type=openapi.TYPE_STRING)
framework_parameter = openapi.Parameter(
    "framework", openapi.IN_QUERY, description="Framework", type=openapi.TYPE_STRING)
models_parameter = openapi.Parameter(
    "models", openapi.IN_QUERY, description="Models in csv format", type=openapi.TYPE_STRING)


class RenderedModelsReponseSerializer(serializers.Serializer):
    """test model"""

    file_name = serializers.CharField()
    file_content = serializers.CharField()


@swagger_auto_schema(
    method="GET",
    manual_parameters=[lang_parameter, framework_parameter, models_parameter],
    responses={200: openapi.Response(
        "get_rendered_models", RenderedModelsReponseSerializer(many=True))},
    security=[{"Bearer": []}],
)
@api_view(["GET"])
def render_models(request):
    """Common api handler for rendering models"""
    lang = request.GET["language"]
    framework = request.GET["framework"]
    models_to_render = request.GET["models"].split(",")
    # print(lang, framework, models_to_render)
    if lang == "TS" and framework == "TypeORM":
        return ts_models(models_to_render)
    if lang == "Python" and framework == "SQLAlchemy":
        return py_sqlachmy_models(models_to_render)
    if lang == "Rust" and framework == "rust-diesel":
        return rust_diesel_models(models_to_render)

    return JsonResponse({"message": lang + " is not supported as of now"}, status=400)


def ts_models(models_to_render):
    """Endpoint handler to get typeorm model files"""

    # models_to_render = request.GET["models"].split(",")

    schemas = get_model_db_schemas(models_to_render, "typeORM")
    # print(schemas)
    rendered_index = render_to_string(
        "index.template", {"data": schemas.items()})

    rendered_models = []
    for key, value in schemas.items():
        # print(value)
        enums_to_render = []
        for field_key, field_value in value["fields"].items():
            if "choices" in field_value:
                # print(field_value["choices"])
                enums_to_render.append(
                    {"name": field_key + "Enum",
                        "choices": field_value["choices"], "trsnaform_enum": field_value["trsnaform_enum"]}
                )

        rendered_model = render_to_string(
            "model.template",
            {
                "fields": value["fields"].items(),
                "docs": value["__doc__"],
                "name": key,
                "table_name": value["table_name"],
                "enums": enums_to_render,
                "model_imports": value["model_imports"],
                "relation_imports": value["relation_imports"],
            },
        )
        # print('-------' , key)
        # print(rendered_model)
        rendered_models.append(
            {"file_name": key + ".Entity.ts", "file_content": rendered_model})
    rendered_models.append(
        {"file_name": "index.ts", "file_content": rendered_index})
    return JsonResponse(rendered_models, safe=False)


def rust_diesel_models(models_to_render):
    """Rust diesel models generator"""
    schemas = get_model_db_schemas(models_to_render, "rust-diesel")
    # print(schemas)
    rendered_files = []

    enums_to_render = []
    relation_tables_to_render = []
    for _key, value in schemas.items():
        for relation_table in value["relation_tables"]:
            relation_tables_to_render.append(relation_table)
        for field_key, field_value in value["fields"].items():
            if "choices" in field_value:
                # print(field_value["choices"])
                enums_to_render.append(
                    {"name": field_key + "Enum",
                        "choices": field_value["choices"], "trsnaform_enum": field_value["trsnaform_enum"]}
                )

    rendered_models = render_to_string(
        "rust-diesel.template",
        {"schemas": schemas, "enums": enums_to_render,
            "relation_tables": relation_tables_to_render},
    )
    rendered_files.append(
        {"file_name": "schema.rs", "file_content": rendered_models})
    rendered_files.append(
        {"file_name": "mod.rs", "file_content": "pub mod schema;"})

    return JsonResponse(rendered_files, safe=False)


def py_sqlachmy_models(models_to_render):
    """Gets sqlalchemy models"""
    # models_to_render = request.GET["models"].split(",")

    schemas = get_model_db_schemas(models_to_render, "py-sqlalchemy")
    # print(schemas)
    rendered_files = []
    # print(value)
    enums_to_render = []
    relation_tables_to_render = []
    for _key, value in schemas.items():
        for relation_table in value["relation_tables"]:
            relation_tables_to_render.append(relation_table)
        for field_key, field_value in value["fields"].items():
            if "choices" in field_value:
                # print(field_value["choices"])
                enums_to_render.append(
                    {"name": field_key + "Enum",
                        "choices": field_value["choices"], "trsnaform_enum": field_value["trsnaform_enum"]}
                )

    rendered_models = render_to_string(
        "sqlalchemy-model.template",
        {"schemas": schemas, "enums": enums_to_render,
            "relation_tables": relation_tables_to_render},
    )
    rendered_files.append({"file_name": "sql_models.py",
                          "file_content": rendered_models})
    return JsonResponse(rendered_files, safe=False)
