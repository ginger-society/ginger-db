from ginger.db import models


class student(models.Model):

    name = models.CharField()

    roll_number = models.CharField()

    on_scholarship = models.BooleanField(
        default=True,
    )

    father_name = models.CharField(
        null=True,
    )

    address = models.TextField()


course_type = (
    ("compulsary", "Compulsary"),
    ("elective", "Elective"),
)


class enrollment(models.Model):

    student = models.ForeignKey(
        student, related_name="courses", on_delete=models.DO_NOTHING
    )

    course = models.ForeignKey(
        course,
    )


class course(models.Model):

    name = models.CharField()

    type = models.CharField(choices=course_type)
