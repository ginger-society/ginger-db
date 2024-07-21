from ginger.db import models


class user(models.Model):
    """User's table"""

    first_name = models.CharField(max_length=40,     blank=True, null=True,)

    last_name = models.CharField(max_length=40,     blank=True, null=True,)

    middle_name = models.CharField(max_length=40,     blank=True, null=True,)

    email_id = models.CharField(max_length=100,)

    mobile_number = models.CharField(max_length=15,     blank=True, null=True,)

    created_at = models.DateTimeField(auto_now_add=True,)

    updated_at = models.DateTimeField(auto_now=True,)

    password_hash = models.CharField(
        max_length=400,     blank=True, null=True,)

    class Meta:
        db_table = "user"


class token(models.Model):
    """tokens like password hashes , TOTP , session tokens etc"""

    session_hash = models.CharField(max_length=400,     blank=True, null=True,)

    user = models.ForeignKey(
        'user', related_name='tokens', on_delete=models.CASCADE,)

    app = models.ForeignKey('app', on_delete=models.CASCADE,)

    class Meta:
        db_table = "token"


class app(models.Model):
    """Application"""

    client_id = models.CharField(max_length=150,)

    name = models.CharField(max_length=50,)

    class Meta:
        db_table = "app"


class group(models.Model):
    """Groups"""

    identifier = models.CharField(max_length=50,)

    users = models.ManyToManyField('user', related_name='groups',)

    owners = models.ManyToManyField('user', related_name='managed_groups',)

    class Meta:
        db_table = "group"
