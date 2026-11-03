using System.Text;
using ScreenKeyboard.Layouts;
using ScreenKeyboard.Text;

namespace ScreenKeyboard.Glide;

/// <summary>A 2D point of a gesture in keyboard coordinates.</summary>
public readonly record struct GesturePoint(double X, double Y);

/// <summary>
/// Statistical glide (swipe) typing classifier. This is a C# port of FlorisBoard's
/// <c>StatisticalGlideTypingClassifier</c> (Apache-2.0), which is based on the SHARK² approach:
/// the user gesture is compared to the ideal gesture of each candidate word by shape (normalized) and
/// location, combined with the word frequency.
/// </summary>
public sealed class GlideTypingClassifier
{
    /// <summary>Allowed length variance (in key radii) before a word is pruned.</summary>
    public const double PruningLengthThreshold = 8.42;

    /// <summary>Number of points gestures are resampled to.</summary>
    public const int SamplingPoints = 200;

    /// <summary>Standard deviation of the shape distance distribution.</summary>
    public const double ShapeStd = 22.08;

    /// <summary>Standard deviation of the location distance distribution (factor of the key radius).</summary>
    public const double LocationStd = 0.5109;

    private readonly Gesture _gesture = new();
    private readonly object _gate = new();
    private Dictionary<char, KeyRect> _keysByChar = new();
    private List<KeyValuePair<char, KeyRect>> _keys = new();
    private IReadOnlyList<(string Word, int Frequency)> _words = [];
    private Dictionary<string, int> _frequencies = new(StringComparer.Ordinal);
    private Pruner? _pruner;
    private double _distanceThresholdSquared;
    private (Gesture Gesture, int Max, IReadOnlyList<string> Result)? _cache;

    /// <summary>Whether both layout and words were provided.</summary>
    public bool IsReady => _pruner is not null;

    /// <summary>Whether the current gesture has no points.</summary>
    public bool IsEmpty => _gesture.IsEmpty;

    /// <summary>Number of points in the current gesture.</summary>
    public int PointCount => _gesture.Count;

    /// <summary>
    /// Sets the key layout. Only character keys whose output is a single letter are used.
    /// </summary>
    public void SetLayout(IEnumerable<ComputedKey> keys)
    {
        var map = new Dictionary<char, KeyRect>();
        foreach (var key in keys)
        {
            var output = key.Data.AsString(false);
            if (output.Length != 1 || !char.IsLetter(output[0]) && output[0] != '\'')
            {
                continue;
            }
            var c = TextUtils.BaseChar(output[0]);
            if (!map.ContainsKey(c))
            {
                map[c] = key.VisibleBounds.IsEmpty ? key.TouchBounds : key.VisibleBounds;
            }
        }
        SetLayout(map);
    }

    /// <summary>Sets the key layout from explicit character → bounds pairs.</summary>
    public void SetLayout(IReadOnlyDictionary<char, KeyRect> keys)
    {
        lock (_gate)
        {
            if (_keysByChar.Count == keys.Count && keys.All(k => _keysByChar.TryGetValue(k.Key, out var r) && r == k.Value))
            {
                return;
            }
            _keysByChar = keys.ToDictionary(k => char.ToLowerInvariant(k.Key), k => k.Value);
            _keys = _keysByChar.ToList();
            if (_keys.Count > 0)
            {
                var width = _keys[0].Value.Width / 4;
                _distanceThresholdSquared = width * width;
            }
            RebuildPruner();
        }
    }

    /// <summary>Sets the candidate words and their frequencies.</summary>
    public void SetWords(IEnumerable<(string Word, int Frequency)> words)
    {
        lock (_gate)
        {
            _words = words.Where(w => w.Word.Length > 1).ToList();
            _frequencies = new Dictionary<string, int>(StringComparer.Ordinal);
            foreach (var (w, f) in _words)
            {
                _frequencies[w] = Math.Max(_frequencies.TryGetValue(w, out var e) ? e : 0, f);
            }
            RebuildPruner();
        }
    }

    /// <summary>Uses all words of a dictionary as candidates.</summary>
    public void SetWords(WordDictionary dictionary) =>
        SetWords(dictionary.Words.Select(w => (w, dictionary.GetFrequency(w))));

    private void RebuildPruner()
    {
        _cache = null;
        _pruner = _keys.Count > 0 && _words.Count > 0 ? new Pruner(PruningLengthThreshold, _words.Select(w => w.Word), _keysByChar) : null;
    }

    /// <summary>Clears the current gesture.</summary>
    public void Clear()
    {
        lock (_gate)
        {
            _gesture.Clear();
        }
    }

    /// <summary>Adds a point to the current gesture (points too close to the previous one are ignored).</summary>
    public void AddPoint(double x, double y)
    {
        lock (_gate)
        {
            if (!_gesture.IsEmpty)
            {
                var dx = _gesture.LastX - x;
                var dy = _gesture.LastY - y;
                if (dx * dx + dy * dy <= _distanceThresholdSquared)
                {
                    return;
                }
            }
            _gesture.Add(x, y);
        }
    }

    /// <summary>Returns the best matching words for the current gesture, best first.</summary>
    public IReadOnlyList<string> GetSuggestions(int maxSuggestionCount)
    {
        lock (_gate)
        {
            if (_pruner is null || _gesture.Count < 2)
            {
                return [];
            }
            if (_cache is { } c && c.Max == maxSuggestionCount && c.Gesture.Equals(_gesture))
            {
                return c.Result;
            }
            var result = Classify(maxSuggestionCount);
            _cache = (_gesture.Clone(), maxSuggestionCount, result);
            return result;
        }
    }

    /// <summary>Classifies a complete gesture.</summary>
    public IReadOnlyList<string> Classify(IEnumerable<GesturePoint> points, int maxSuggestionCount)
    {
        lock (_gate)
        {
            _gesture.Clear();
            foreach (var p in points)
            {
                AddPoint(p.X, p.Y);
            }
            return GetSuggestions(maxSuggestionCount);
        }
    }

    private IReadOnlyList<string> Classify(int maxSuggestionCount)
    {
        var candidates = new List<string>();
        var weights = new List<double>();
        if (_keys.Count == 0 || _pruner is null)
        {
            return candidates;
        }
        var firstKey = _keys[0].Value;
        var radius = Math.Min(firstKey.Height, firstKey.Width);
        var remaining = _pruner.PruneByExtremities(_gesture, _keys);
        var userGesture = _gesture.Resample(SamplingPoints);
        var normalizedUser = userGesture.NormalizeByBoxSide();
        remaining = _pruner.PruneByLength(_gesture, remaining, _keysByChar, radius);

        foreach (var word in remaining)
        {
            foreach (var ideal in Gesture.GenerateIdealGestures(word, _keysByChar))
            {
                var wordGesture = ideal.Resample(SamplingPoints);
                var normalized = wordGesture.NormalizeByBoxSide();
                var shapeDistance = ShapeDistance(normalized, normalizedUser);
                var locationDistance = LocationDistance(wordGesture, userGesture);
                var shapeProbability = Gaussian(shapeDistance, 0, ShapeStd);
                var locationProbability = Gaussian(locationDistance, 0, LocationStd * radius);
                var frequency = Math.Max(1, _frequencies.TryGetValue(word, out var f) ? f : 1);
                var confidence = 1.0 / (shapeProbability * locationProbability * frequency);

                var index = 0;
                var duplicateIndex = int.MaxValue;
                while (index < weights.Count && weights[index] <= confidence)
                {
                    if (candidates[index] == word)
                    {
                        duplicateIndex = index;
                    }
                    index++;
                }
                if (index < maxSuggestionCount && index <= duplicateIndex)
                {
                    if (duplicateIndex < int.MaxValue)
                    {
                        weights.RemoveAt(duplicateIndex);
                        candidates.RemoveAt(duplicateIndex);
                    }
                    weights.Insert(index, confidence);
                    candidates.Insert(index, word);
                    if (weights.Count > maxSuggestionCount)
                    {
                        weights.RemoveAt(maxSuggestionCount);
                        candidates.RemoveAt(maxSuggestionCount);
                    }
                }
            }
        }
        return candidates;
    }

    private static double LocationDistance(Gesture a, Gesture b)
    {
        double total = 0;
        for (var i = 0; i < SamplingPoints; i++)
        {
            total += Math.Abs(a.GetX(i) - b.GetX(i)) + Math.Abs(a.GetY(i) - b.GetY(i));
        }
        return total / SamplingPoints / 2;
    }

    private static double ShapeDistance(Gesture a, Gesture b)
    {
        double total = 0;
        for (var i = 0; i < SamplingPoints; i++)
        {
            total += Gesture.Distance(a.GetX(i), a.GetY(i), b.GetX(i), b.GetY(i));
        }
        return total;
    }

    private static double Gaussian(double value, double mean, double std)
    {
        var factor = 1.0 / (std * Math.Sqrt(2 * Math.PI));
        var exponent = Math.Pow((value - mean) / std, 2);
        return Math.Max(double.Epsilon, factor * Math.Exp(-0.5 * exponent));
    }

    private sealed class Pruner
    {
        private readonly double _lengthThreshold;
        private readonly Dictionary<(char, char), List<string>> _wordTree = new();
        private readonly Dictionary<string, double> _idealLengths = new(StringComparer.Ordinal);

        public Pruner(double lengthThreshold, IEnumerable<string> words, Dictionary<char, KeyRect> keysByChar)
        {
            _lengthThreshold = lengthThreshold;
            foreach (var word in words)
            {
                var first = TextUtils.BaseChar(word[0]);
                var last = TextUtils.BaseChar(word[word.Length - 1]);
                if (!keysByChar.ContainsKey(first) || !keysByChar.ContainsKey(last))
                {
                    continue;
                }
                if (!_wordTree.TryGetValue((first, last), out var list))
                {
                    list = new List<string>();
                    _wordTree[(first, last)] = list;
                }
                list.Add(word);
            }
        }

        public List<string> PruneByExtremities(Gesture gesture, List<KeyValuePair<char, KeyRect>> keys)
        {
            var result = new List<string>();
            var startKeys = FindClosestKeys(gesture.FirstX, gesture.FirstY, 2, keys);
            var endKeys = FindClosestKeys(gesture.LastX, gesture.LastY, 2, keys);
            foreach (var s in startKeys)
            {
                foreach (var e in endKeys)
                {
                    if (_wordTree.TryGetValue((s, e), out var words))
                    {
                        result.AddRange(words);
                    }
                }
            }
            return result;
        }

        public List<string> PruneByLength(Gesture gesture, List<string> words, Dictionary<char, KeyRect> keysByChar, double radius)
        {
            var result = new List<string>();
            var userLength = gesture.Length;
            foreach (var word in words)
            {
                foreach (var ideal in Gesture.GenerateIdealGestures(word, keysByChar))
                {
                    if (!_idealLengths.TryGetValue(word, out var length))
                    {
                        length = ideal.Length;
                        _idealLengths[word] = length;
                    }
                    if (Math.Abs(userLength - length) < _lengthThreshold * radius)
                    {
                        result.Add(word);
                    }
                }
            }
            return result;
        }

        private static IEnumerable<char> FindClosestKeys(double x, double y, int n, List<KeyValuePair<char, KeyRect>> keys) =>
            keys.OrderBy(k => Gesture.Distance(k.Value.CenterX, k.Value.CenterY, x, y)).Take(n).Select(k => k.Key);
    }

    internal sealed class Gesture
    {
        private const int MaxSize = 500;
        private readonly double[] _xs;
        private readonly double[] _ys;

        public Gesture() : this(new double[MaxSize], new double[MaxSize], 0)
        {
        }

        private Gesture(double[] xs, double[] ys, int count)
        {
            _xs = xs;
            _ys = ys;
            Count = count;
        }

        public int Count { get; private set; }
        public bool IsEmpty => Count == 0;
        public double FirstX => Count > 0 ? _xs[0] : 0;
        public double FirstY => Count > 0 ? _ys[0] : 0;
        public double LastX => Count > 0 ? _xs[Count - 1] : 0;
        public double LastY => Count > 0 ? _ys[Count - 1] : 0;

        public double GetX(int i) => i < Count ? _xs[i] : 0;
        public double GetY(int i) => i < Count ? _ys[i] : 0;

        public void Add(double x, double y)
        {
            if (Count >= MaxSize)
            {
                return;
            }
            _xs[Count] = x;
            _ys[Count] = y;
            Count++;
        }

        public void Clear() => Count = 0;

        public Gesture Clone() => new((double[])_xs.Clone(), (double[])_ys.Clone(), Count);

        public double Length
        {
            get
            {
                double length = 0;
                for (var i = 1; i < Count; i++)
                {
                    length += Distance(_xs[i - 1], _ys[i - 1], _xs[i], _ys[i]);
                }
                return length;
            }
        }

        public static double Distance(double x1, double y1, double x2, double y2)
        {
            var dx = x1 - x2;
            var dy = y1 - y2;
            return Math.Sqrt(dx * dx + dy * dy);
        }

        public Gesture Resample(int numPoints)
        {
            var result = new Gesture();
            if (Count == 0)
            {
                return result;
            }
            var interpointDistance = Length / numPoints;
            result.Add(_xs[0], _ys[0]);
            var lastX = _xs[0];
            var lastY = _ys[0];
            double cumulativeError = 0;
            if (Count == 1 || interpointDistance <= 0)
            {
                for (var i = 0; i < SamplingPoints; i++)
                {
                    result.Add(_xs[0], _ys[0]);
                }
                return result;
            }
            for (var i = 0; i < Count - 1; i++)
            {
                var dx = _xs[i + 1] - _xs[i];
                var dy = _ys[i + 1] - _ys[i];
                var norm = Math.Sqrt(dx * dx + dy * dy);
                if (norm <= 0)
                {
                    continue;
                }
                dx /= norm;
                dy /= norm;
                var numNewPoints = norm / interpointDistance;
                cumulativeError += numNewPoints - Math.Floor(numNewPoints);
                if (cumulativeError > 1)
                {
                    numNewPoints = Math.Floor(numNewPoints) + Math.Floor(cumulativeError);
                    cumulativeError %= 1;
                }
                for (var j = 0; j < (int)numNewPoints; j++)
                {
                    lastX += dx * interpointDistance;
                    lastY += dy * interpointDistance;
                    result.Add(lastX, lastY);
                }
            }
            return result;
        }

        public Gesture NormalizeByBoxSide()
        {
            var result = new Gesture();
            double maxX = double.MinValue, maxY = double.MinValue, minX = double.MaxValue, minY = double.MaxValue;
            for (var i = 0; i < Count; i++)
            {
                maxX = Math.Max(_xs[i], maxX);
                maxY = Math.Max(_ys[i], maxY);
                minX = Math.Min(_xs[i], minX);
                minY = Math.Min(_ys[i], minY);
            }
            var width = maxX - minX;
            var height = maxY - minY;
            var longest = Math.Max(Math.Max(width, height), 0.00001);
            var cx = (width / 2 + minX) / longest;
            var cy = (height / 2 + minY) / longest;
            for (var i = 0; i < Count; i++)
            {
                result.Add(_xs[i] / longest - cx, _ys[i] / longest - cy);
            }
            return result;
        }

        public static List<Gesture> GenerateIdealGestures(string word, Dictionary<char, KeyRect> keysByChar)
        {
            var ideal = new Gesture();
            var withLoops = new Gesture();
            var previous = '\0';
            var hasLoops = false;
            foreach (var ch in word)
            {
                var lc = char.ToLowerInvariant(ch);
                if (!keysByChar.TryGetValue(lc, out var key) && !keysByChar.TryGetValue(TextUtils.BaseChar(lc), out key))
                {
                    continue;
                }
                var cx = key.CenterX;
                var cy = key.CenterY;
                if (previous == lc)
                {
                    // Small loop on repeated letters to distinguish words like "pool" and "pol".
                    withLoops.Add(cx + key.Width / 4, cy + key.Height / 4);
                    withLoops.Add(cx + key.Width / 4, cy - key.Height / 4);
                    withLoops.Add(cx - key.Width / 4, cy - key.Height / 4);
                    withLoops.Add(cx - key.Width / 4, cy + key.Height / 4);
                    hasLoops = true;
                    ideal.Add(cx, cy);
                }
                else
                {
                    ideal.Add(cx, cy);
                    withLoops.Add(cx, cy);
                }
                previous = lc;
            }
            return hasLoops ? [ideal, withLoops] : [ideal];
        }

        public override bool Equals(object? obj)
        {
            if (obj is not Gesture other || other.Count != Count)
            {
                return false;
            }
            for (var i = 0; i < Count; i++)
            {
                if (_xs[i] != other._xs[i] || _ys[i] != other._ys[i])
                {
                    return false;
                }
            }
            return true;
        }

        public override int GetHashCode() => Count;
    }
}
