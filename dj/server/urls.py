"""
URL configuration for server project.

The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.gingersociety.org/ginger-dj/en/4.2/topics/http/urls/
Examples:
Function views
    1. Add an import:  from my_app import views
    2. Add a URL to urlpatterns:  path('', views.home, name='home')
Class-based views
    1. Add an import:  from other_app.views import Home
    2. Add a URL to urlpatterns:  path('', Home.as_view(), name='home')
Including another URLconf
    1. Import the include() function: from ginger.urls import include, path
    2. Add a URL to urlpatterns:  path('blog/', include('blog.urls'))
"""

from ginger.contrib import admin
from ginger.urls import include, path
from src.views import *

urlpatterns = [
    path("admin/", admin.site.urls),
    path("models/", get_model_schema),
    path("py-sqlalchemy-models/", get_sqlalchemy_model_schema),
    path("rust-diesel-models/", get_diesel_model_schema),
    path("api/render_models", render_models),
    path("api/get-all-models", get_all_defined_models),
    path("", include("ginger.prometheus.urls")),
]
