using System.Threading.Tasks;

namespace VolumeProfileManager.Cli;

public interface ICliHost
{
    Task ExecuteAsync(string[] args);
}
