from ginger.db import models


course_type = (

    ("compulsary", "Compulsary"),

    ("elective", "Elective"),

)


class student(models.Model):
    """Lorem ipsum dolem text"""

    name = models.CharField(max_length=150,)

    roll_number = models.CharField(max_length=40,)

    on_scholarship = models.BooleanField(default=True,)

    father_name = models.CharField(max_length=100,     null=True,)

    address = models.TextField(max_length=500,)

    data_of_birth = models.DateField(null=True,)

    created_at = models.DateTimeField(auto_now_add=True,)

    updated_at = models.DateField(auto_now=True,)

    has_cab_service = models.BooleanField(default=False,      null=True,)

    class Meta:
        db_table = "student"


class enrollment(models.Model):
    """Once again the lorem ipsum text"""

    student = models.ForeignKey(
        'student', related_name='courses', on_delete=models.DO_NOTHING,)

    course = models.ForeignKey(
        'course', on_delete=models.SET_NULL,      null=True,)

    class Meta:
        db_table = "enrollment"


class course(models.Model):
    """Another lorem ipsum dolem text"""

    name = models.CharField(max_length=100,)

    type = models.CharField(choices=course_type,
                            max_length=50,   default='compulsary',)

    class Meta:
        db_table = "course"
