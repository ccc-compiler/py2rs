class Person:
    def __init__(self, name: str, age: int) -> None:
        self.name = name
        self.age = age

    def role(self) -> str:
        return "person"

    def intro(self) -> str:
        return f"I am {self.name}, {self.age}, a {self.role()}"


class Student(Person):
    def __init__(self, name: str, age: int, year: int) -> None:
        super().__init__(name, age)
        self.year = year
        self.grades: dict[str, list[int]] = {}

    def role(self) -> str:
        return f"year-{self.year} student"

    def add_grade(self, course: str, g: int) -> None:
        if course not in self.grades:
            self.grades[course] = []
        self.grades[course].append(g)

    def average(self) -> float:
        all_g = [g for gs in self.grades.values() for g in gs]
        if not all_g:
            return 0.0
        return sum(all_g) / len(all_g)


class Teacher(Person):
    def __init__(self, name: str, age: int, subject: str) -> None:
        super().__init__(name, age)
        self.subject = subject

    def role(self) -> str:
        return f"{self.subject} teacher"

    def intro(self) -> str:
        return super().intro() + "; office hours on Friday"


class Course:
    def __init__(self, code: str, teacher: str, cap: int) -> None:
        self.code = code
        self.teacher = teacher
        self.cap = cap
        self.roster: list[str] = []

    def enroll(self, student: str) -> bool:
        if len(self.roster) >= self.cap or student in self.roster:
            return False
        self.roster.append(student)
        return True


def letter(avg: float) -> str:
    if avg >= 90:
        return "A"
    if avg >= 80:
        return "B"
    if avg >= 70:
        return "C"
    return "D"


def main() -> None:
    teachers = [Teacher("Ms. Kim", 41, "math"), Teacher("Mr. Ode", 35, "history")]
    students = [Student("ana", 15, 1), Student("ben", 16, 2), Student("cho", 15, 1), Student("dev", 17, 3)]
    for t in teachers:
        print(t.intro())
    for s in students:
        print(s.intro())
    courses = [Course("MATH1", "Ms. Kim", 3), Course("HIST1", "Mr. Ode", 2)]
    for c in courses:
        for s in students:
            ok = c.enroll(s.name)
            print(f"{c.code}: {s.name} {'enrolled' if ok else 'rejected'}")
    for i, s in enumerate(students):
        for j, c in enumerate(courses):
            if s.name in c.roster:
                for k in range(3):
                    s.add_grade(c.code, 60 + (i * 13 + j * 7 + k * 11) % 40)
    for s in sorted(students, key=lambda s: -s.average()):
        print(f"{s.name:<5}{s.average():6.1f} {letter(s.average())} {s.grades}")
    people: list[Person] = [Teacher("Dr. Fox", 58, "biology"), Student("eli", 16, 2), Person("guest", 30)]
    for p in people:
        print(p.intro())
    print(sum(p.age for p in people) / len(people))
    for p in people:
        if isinstance(p, Teacher):
            print("teacher:", p.name, p.subject)
        elif isinstance(p, Student):
            print("student:", p.name, p.year)


main()
