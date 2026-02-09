#!/usr/bin/env python3
import sys
import os
import numpy as np

def get_locpot_min_max(file_path):
    if not os.path.exists(file_path):
        print(f"Error: File {file_path} not found.")
        return None, None

    try:
        with open(file_path, 'r') as f:
            # Skip first 2 lines
            f.readline() # comment
            f.readline() # scale
            
            # 3 lines of lattice vectors
            f.readline()
            f.readline()
            f.readline()
            
            # Elements (VASP 5 format has element names, VASP 4 doesn't)
            line = f.readline().strip()
            parts = line.split()
            if parts[0].isdigit():
                # VASP 4 format: this line is already the number of atoms
                num_atoms = sum(int(x) for x in parts)
            else:
                # VASP 5 format: this line is element names, next line is number of atoms
                line = f.readline()
                num_atoms = sum(int(x) for x in line.split())
            
            # Check for Selective dynamics or Direct/Cartesian
            line = f.readline().strip()
            if line.lower().startswith('s'):
                # Selective dynamics, next line is Direct/Cartesian
                f.readline()
            
            # Skip atomic coordinates
            for _ in range(num_atoms):
                f.readline()
                
            # Skip potential blank line(s) and find grid dimensions
            line = f.readline().strip()
            while not line:
                line = f.readline().strip()
            
            grid_dims = [int(x) for x in line.split()]
            total_points = grid_dims[0] * grid_dims[1] * grid_dims[2]
            
            # Now read the data. Data is usually 5 numbers per line.
            # Using np.fromfile or reading all and using np.fromstring
            
            data = []
            count = 0
            while count < total_points:
                line = f.readline()
                if not line:
                    break
                parts = line.split()
                for p in parts:
                    try:
                        data.append(float(p))
                        count += 1
                    except ValueError:
                        pass
                if count >= total_points:
                    break
            
            if not data:
                return None, None
            
            data_np = np.array(data)
            return data_np.min(), data_np.max()
            
    except Exception as e:
        print(f"Error parsing LOCPOT: {e}")
        return None, None

def main():
    if len(sys.argv) < 2:
        file_path = "LOCPOT"
    else:
        file_path = sys.argv[1]
        
    v_min, v_max = get_locpot_min_max(file_path)
    if v_min is not None:
        print(f"LOCPOT Min: {v_min:.6f}")
        print(f"LOCPOT Max: {v_max:.6f}")

if __name__ == "__main__":
    main()
