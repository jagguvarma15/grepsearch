def authenticate(username, password):
    return username == "admin" and password == "secret"


def greet(name):
    print("hello " + name)
