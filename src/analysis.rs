mod utils {
    use std::{io::Write, fs::File};

    pub fn write_val_vec_to_file<T: std::fmt::Debug> (list: &Vec<T>, dir: &String) {
        let mut f = File::create(dir).unwrap();
        for fpr in list {
            let output = format!("{:?}\n", fpr);
            let _ = f.write_all(&output.as_bytes());
        }
    }
}

mod traceability {
    use std::{collections::HashMap, io::{BufReader, BufRead}, f64, fs};

    pub fn import_csv(file_dir: &String) -> HashMap<usize, (usize,f64)> {
        let file = fs::File::open(file_dir).unwrap();
        let reader = BufReader::new(file);
        let mut id_val_map = HashMap::<usize, (usize,f64)>::new();
        for line in reader.lines() {
            let str_line = line.unwrap();
            let items: Vec<&str> = str_line.split(",").collect();
            (items[0] != "id").then(|| {
                let id = items[0].to_string().parse::<usize>().unwrap();
                let inf = items[3].to_string().parse::<usize>().unwrap();
                let fuz = items[2].to_string().parse::<f64>().unwrap();
                id_val_map.insert(id, (inf,fuz));
            });
        }
        id_val_map
    }

    pub fn find_thd_fpr(hmap: &HashMap<usize,(usize,f64)>) -> Vec::<(f64, f64)> {
        let max_inf = find_max_inf_level(&hmap);
        let mut thd_fpr_list = Vec::<(f64, f64)>::new();
        let mut fuz_list: Vec<f64> = hmap.iter()
            .filter(|(_,(inf,_))| *inf == max_inf)
            .map(|(_, (_,fuz))| *fuz)
            .collect();

        fuz_list.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut i = 0;
        while *fuz_list.get(i).unwrap() != 100.00 {
            (i != 0).then(|| {
                while *fuz_list.get(i).expect(&i.to_string()) == *fuz_list.get(i-1).unwrap() {
                    i += 1;
                }
            });

            let fuz_min: f64 = *fuz_list.get(i).unwrap();
            let inf_list: Vec<usize> = hmap.iter()
                .filter(|(_,(_,fuz))| *fuz >= fuz_min)
                .map(|(_, (inf,_))| *inf)
                .collect();

            let fpr = inf_list.iter().filter(|inf| **inf == 0).count() as f64 / inf_list.len() as f64;

            i += 1;
            thd_fpr_list.push((fuz_min, fpr));
        }
        thd_fpr_list
    }

    pub fn calc_thd_fpr(hmap: &HashMap<usize,(usize,f64)>, thd_list: &Vec<f64>) -> Vec::<f64> {
        let mut fpr_list = Vec::<f64>::new();

        thd_list.iter().for_each(|thd| {
            let inf_list: Vec<usize> = hmap.iter()
            .filter(|(_,(_,fuz))| *fuz >= *thd)
            .map(|(_, (inf,_))| *inf)
            .collect();
        
            let fpr = match inf_list.len() == 0 {
                true => 0.0,
                false => inf_list.iter().filter(|inf| **inf == 0).count() as f64 / inf_list.len() as f64,
            };
            // avoid 
            if (*thd == 99.99) & (fpr == 0.0) {
                panic!("Find zero?")
            }
            fpr_list.push(fpr);
        });
        fpr_list
    }

    pub fn calc_inf_dist(hmap: &HashMap<usize,(usize,f64)>) -> Vec::<usize> {
        let max_inf = find_max_inf_level(hmap);
        let mut inf_list = Vec::<usize>::new();

        for i in 0..max_inf {
            let count: usize = hmap.iter()
                .filter(|(_, (inf,_))| *inf == i + 1)
                .map(|(_, (inf,_))| *inf)
                .collect::<Vec<usize>>()
                .len();
            inf_list.push(count);
        }
        inf_list
    }

    pub fn calc_inf_fpr_dist_in_thd(hmap: &HashMap<usize,(usize,f64)>, thd_list: &Vec<f64>) -> Vec::<Vec<f64>> {
        let max_inf = find_max_inf_level(hmap);
        let real_inf_list = calc_inf_dist(hmap);
        let mut thd_fill_rate_list = Vec::<Vec<f64>>::new();
        
        thd_list.iter().for_each(|thd| {
            let mut fill_rate_list = Vec::<f64>::new();
            let mut thd_inf_list = Vec::<usize>::new();
            for i in 0..max_inf {
                let count: usize = hmap.iter()
                    .filter(|(_,(_,fuz))| *fuz >= *thd)
                    .filter(|(_, (inf,_))| *inf == i + 1)
                    .map(|(_, (inf,_))| *inf)
                    .collect::<Vec<usize>>()
                    .len();
                thd_inf_list.push(count);
            }

            for i in 0..max_inf {
                let fill_rate = (*thd_inf_list.get(i).unwrap() as f64) / (*real_inf_list.get(i).unwrap() as f64);
                fill_rate_list.push(fill_rate);
            }

            thd_fill_rate_list.push(fill_rate_list);
        });
        thd_fill_rate_list
    }

    fn find_max_inf_level(hmap: &HashMap<usize,(usize,f64)>) -> usize {
        let inf_list: Vec<usize> = hmap.iter()
                .map(|(_, (inf,_))| *inf)
                .collect();
        inf_list.into_iter().max().unwrap()
    }

    pub fn gen_thd_list() -> Vec::<f64>{
        let mut thd_list = Vec::<f64>::new();
        let (mut lower_bound, upper_bound, step_1, step_2) = (99.99, 100.0001, 0.001, 0.0002);
        while lower_bound < upper_bound {
            let input = f64::trunc(lower_bound * 10000.0) / 10000.0;
            thd_list.push(input);
            if lower_bound < 99.999 {
                lower_bound += step_1;
            }
            else {
                lower_bound += step_2;
            }
        }
        thd_list
    }

}

mod tests {
    use crate::analysis::traceability::calc_inf_fpr_dist_in_thd;

    use super::{traceability::{find_thd_fpr, import_csv, calc_thd_fpr, calc_inf_dist, gen_thd_list}, utils::write_val_vec_to_file};

    extern crate test;

    #[test]
    fn test_thd_fpr(){
        let thd_list = gen_thd_list();
        let val_list = import_csv(&"../Traceability-Evaluation/inputs/fuz_val_and_inf.csv".to_string());
        write_val_vec_to_file(&calc_thd_fpr(&val_list, &thd_list), &"./output/thd_fpr_fix_step/thd_fpr.txt".to_string());
    }

    #[test]
    fn test_inf_dist(){
        let list = import_csv(&"../Traceability-Evaluation/inputs/fuz_val_and_inf.csv".to_string());
        write_val_vec_to_file(&calc_inf_dist(&list), &"./output/inf_dist/k_shell.txt".to_string());
    }

    #[test]
    fn test_calc_inf_fpr_dist_in_thd() {
        let thd_list = gen_thd_list();
        let list = import_csv(&"../Traceability-Evaluation/inputs/fuz_val_and_inf.csv".to_string());
        write_val_vec_to_file(&calc_inf_fpr_dist_in_thd(&list, &thd_list), &"./output/inf_fpr_dist/inf_fpr.txt".to_string());
    }
}