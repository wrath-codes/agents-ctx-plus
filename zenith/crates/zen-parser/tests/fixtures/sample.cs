using System;
using System.Collections.Generic;

namespace Zenith.Sample
{
    public delegate int Transformer(int x);

    public interface IData
    {
        int Value { get; }
        event EventHandler Changed;
        int this[int index] { get; }
    }

    public struct DataPoint
    {
        public int X;
        public int Y;
    }

    public enum Status
    {
        New,
        Ready,
        Done,
    }

    public class Widget : IData
    {
        public const int MaxSize = 16;
        internal static int _count = 0;
        public event EventHandler Changed;
        public string Title { get; set; }

        public int this[int index] => index;

        public Widget(string title)
        {
            Title = title;
        }

        public void Render() { }

        public static Widget operator +(Widget left, Widget right) => left;

        public static explicit operator int(Widget item) => item.Title.Length;
    }
}
