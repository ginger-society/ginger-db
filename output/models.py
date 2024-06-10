from ginger.db import models



class student(models.Model):
    
    pk = models.BigAutoField() 
    name = models.CharField() 
    roll_number = models.CharField() 
    on_scholarship = models.BooleanField() 
    father_name = models.CharField(null=True, ) 
    address = models.TextField() 




course_type = (
    
        ("compulsary", "Compulsary"),
    
        ("elective", "Elective"),
    
)




class enrollment(models.Model):
    
    pk = models.BigAutoField() 
    student = models.ForeignKey(on_delete=models.DO_NOTHING) 
    course = models.ForeignKey() 




class course(models.Model):
    
    pk = models.BigAutoField() 
    name = models.CharField() 
    type = models.CharField() 



