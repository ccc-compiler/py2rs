class Vehicle:
    def __init__(self, make: str, wheels: int, speed: float) -> None:
        self.make = make
        self.wheels = wheels
        self.speed = speed
        self.odometer = 0.0

    def fuel_per_km(self) -> float:
        return 0.0

    def drive(self, hours: float) -> float:
        km = self.speed * hours
        self.odometer += km
        return km

    def cost(self, km: float) -> float:
        return km * self.fuel_per_km() * 1.8

    def __repr__(self) -> str:
        return f"{type_name(self)}({self.make}, odo={self.odometer:.1f})"


class Car(Vehicle):
    def __init__(self, make: str, seats: int) -> None:
        super().__init__(make, 4, 90.0)
        self.seats = seats

    def fuel_per_km(self) -> float:
        return 0.07


class Truck(Vehicle):
    def __init__(self, make: str, load: float) -> None:
        super().__init__(make, 6, 70.0)
        self.load = load

    def fuel_per_km(self) -> float:
        return 0.2 + self.load * 0.01

    def drive(self, hours: float) -> float:
        if self.load > 20.0:
            hours = hours * 0.8
        return super().drive(hours)


class Bicycle(Vehicle):
    def __init__(self, make: str) -> None:
        super().__init__(make, 2, 18.0)


class ElectricCar(Car):
    def __init__(self, make: str, seats: int, battery: float) -> None:
        super().__init__(make, seats)
        self.battery = battery

    def fuel_per_km(self) -> float:
        return 0.0

    def cost(self, km: float) -> float:
        return km * 0.15 * 0.3

    def drive(self, hours: float) -> float:
        max_km = self.battery * 6.0
        km = min(self.speed * hours, max_km)
        self.battery -= km / 6.0
        self.odometer += km
        return km


def type_name(v: Vehicle) -> str:
    if isinstance(v, ElectricCar):
        return "ElectricCar"
    if isinstance(v, Car):
        return "Car"
    if isinstance(v, Truck):
        return "Truck"
    if isinstance(v, Bicycle):
        return "Bicycle"
    return "Vehicle"


def main() -> None:
    fleet: list[Vehicle] = [
        Car("toyota", 5),
        Truck("volvo", 12.0),
        Truck("man", 30.0),
        Bicycle("giant"),
        ElectricCar("tesla", 5, 40.0),
    ]
    trips = [1.5, 2.0, 0.5]
    total_cost = 0.0
    for day, h in enumerate(trips):
        print(f"day {day + 1}: {h} hours")
        for v in fleet:
            km = v.drive(h)
            c = v.cost(km)
            total_cost += c
            print(f"  {type_name(v):<12}{v.make:<8}{km:>8.1f} km {c:>8.2f}")
    print(fleet)
    print(f"total cost {total_cost:.2f}")
    wheels = sum(v.wheels for v in fleet)
    print("wheels:", wheels)
    far = sorted(fleet, key=lambda v: -v.odometer)
    print([v.make for v in far])
    for v in fleet:
        if isinstance(v, ElectricCar):
            print(f"{v.make} battery left {v.battery:.2f}")
        if isinstance(v, Car):
            print(f"{v.make} seats {v.seats}")


main()
