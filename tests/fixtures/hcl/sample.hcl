# Web instance.
resource "aws_instance" "web" {
  ami           = "ami-123456"
  instance_type = "t2.micro"
  user_data     = "this user data string is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output for the hcl fixture plus padding appended to comfortably exceed the limit"

  tags = {
    Name = "web"
  }
}
