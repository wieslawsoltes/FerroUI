import QtQml
import "./fetched.mjs" as MathLibrary
QtObject { property int value: MathLibrary.answer(6) }
