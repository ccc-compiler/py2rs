import geometry
from geometry import Point, perimeter
from textutil import label


def main() -> None:
    square = [Point(0, 0), Point(1, 0), Point(1, 1), Point(0, 1)]
    print(label(perimeter(square)))
    p: geometry.Point = geometry.Point(3.0, 4.0)
    print(label(geometry.dist(Point(0.0, 0.0), p)), geometry.UNIT)


if __name__ == "__main__":
    main()
