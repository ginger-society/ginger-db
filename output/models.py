from ginger.db import models


course_type = (
    ("compulsary", "Compulsary"),
    ("elective", "Elective"),
)


class student(models.Model):

    name = models.CharField(
        max_length=150,
    )

    roll_number = models.CharField(
        max_length=40,
    )

    on_scholarship = models.BooleanField(
        default=True,
    )

    father_name = models.CharField(
        max_length=100,
        null=True,
    )

    address = models.TextField(
        max_length=500,
    )

    class Meta:
        db_table = "student"


class enrollment(models.Model):

    student = models.ForeignKey(
        "student",
        related_name="courses",
        on_delete=models.DO_NOTHING,
    )

    course = models.ForeignKey(
        "course",
        on_delete=models.SET_NULL,
        null=True,
    )

    class Meta:
        db_table = "enrollment"


class course(models.Model):

    name = models.CharField(
        max_length=100,
    )

    type = models.CharField(
        choices=course_type,
        max_length=50,
        default="compulsary",
    )

    class Meta:
        db_table = "course"
