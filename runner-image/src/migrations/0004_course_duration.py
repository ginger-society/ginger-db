# Generated by Ginger 5.3.4 on 2024-06-10 16:55

from ginger.db import migrations, models


class Migration(migrations.Migration):

    dependencies = [
        ('src', '0003_alter_student_has_cab_service'),
    ]

    operations = [
        migrations.AddField(
            model_name='course',
            name='duration',
            field=models.PositiveIntegerField(null=True),
        ),
    ]