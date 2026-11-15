using DitherKit.IR;
using Xunit;

namespace DitherKit.IR.Tests;
public sealed class ProgramMetadataTests
{
    [Fact]
    public void FirstBloomIncludesZeroRadiusAndIgnoresTables()
    {
        Assert.Equal(1, DitherProgram.FindBloomIndex([0]));
        Assert.Equal(3, DitherProgram.FindBloomIndex([2, 99, 2, 14, 4]));
        Assert.Equal(1, DitherProgram.FindBloomIndex([4, 14, 4, 0, 0]));
        Assert.Equal(1, DitherProgram.FindBloomIndex([4, 14.5f, 4, 0, 0]));
        Assert.Equal(5, DitherProgram.FindBloomIndex([4, 15, 4, 0, 0]));
        Assert.Equal(3, DitherProgram.FindBloomIndex([10, 99, 2, 14, 4, 0, 0, 14, 4, 1, 0.5f]));
    }
    [Fact]
    public void ShaderWalkLimitIsPreserved()
    {
        foreach (int preceding in new[] { 65_535, 65_536 })
        {
            List<float> data = [preceding * 2 + 4];
            for (int i = 0; i < preceding; i++) data.AddRange([99, 2]);
            int split = data.Count;
            data.AddRange([14, 4, 0, 0]);
            Assert.Equal(preceding < 65_536 ? split : data.Count, DitherProgram.FindBloomIndex(data.ToArray()));
        }
    }
    [Fact]
    public void InvalidMetadataDoesNotReadBeyondOperations()
    {
        float[][] lists = [[], [float.NaN], [float.PositiveInfinity], [-1], [1, 99],
            [2, 99, 0], [2, 99, 2.5f], [6, 14, 4, 0, 0, 99, 0]];
        foreach (var data in lists) Assert.Null(DitherProgram.FindBloomIndex(data));
    }
}
