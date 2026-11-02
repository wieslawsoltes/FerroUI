import QtQml
import QtQml.Models

QtObject {
    property ListModel inventory: ListModel {
        id: stock
        ListElement { name: "A"; price: 1 }
        ListElement { name: "B"; price: 3 }
    }
    property string firstName: stock.count ? stock.get(0).name : ""
    function edit(): string {
        stock.append({name: "C", price: 5})
        stock.insert(1, {name: "X", price: 6})
        stock.set(0, {price: 7})
        stock.setProperty(3, "price", 4)
        stock.get(0).price += 1
        stock.move(3, 0, 1)
        stock.remove(2)
        return stock.count + "/" + stock.get(0).name + "/" + stock.get(0).price + "/" + stock.get(1).price
    }
    function invalid() {
        stock.clear()
        stock.insert(0, {name: "replacement", price: 11})
        missingFunction()
    }
}
